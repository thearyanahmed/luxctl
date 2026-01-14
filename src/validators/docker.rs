//! Docker-based validators for language-specific features
//!
//! These validators run inside Docker containers to provide language-specific
//! validation capabilities (e.g., Go race detector, compiler checks).

use crate::tasks::TestCase;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

const DEFAULT_TIMEOUT_SECS: u64 = 120;
const GO_IMAGE: &str = "golang:1.22-alpine";

/// Validator: Run Go race detector on user's code
///
/// This validator:
/// 1. Mounts user's code into a Go container
/// 2. Builds with `-race` flag
/// 3. Runs the binary under concurrent load
/// 4. Checks for race condition reports in stderr
pub struct RaceDetectorValidator {
    pub source_dir: String,
    pub expected_clean: bool,
    pub timeout_secs: u64,
    pub concurrent_requests: u32,
    pub port: u16,
}

impl RaceDetectorValidator {
    pub fn new(expected_clean: bool) -> Self {
        Self {
            source_dir: ".".to_string(),
            expected_clean,
            timeout_secs: DEFAULT_TIMEOUT_SECS,
            concurrent_requests: 50,
            port: 8080,
        }
    }

    pub fn with_source_dir(mut self, dir: &str) -> Self {
        self.source_dir = dir.to_string();
        self
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = secs;
        self
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        // check if docker is available
        if !is_docker_available().await {
            return Ok(TestCase {
                name: "race detector".to_string(),
                result: Err("docker not available - required for race detector".to_string()),
            });
        }

        // get absolute path to source directory
        let source_path = std::fs::canonicalize(&self.source_dir)
            .map_err(|e| format!("cannot resolve source dir '{}': {}", self.source_dir, e))?;

        let source_path_str = source_path.to_string_lossy();

        // step 1: build with race detector inside container
        let build_result = self.docker_build_with_race(&source_path_str).await?;

        if !build_result.success {
            return Ok(TestCase {
                name: "race detector build".to_string(),
                result: Err(format!("build failed: {}", build_result.stderr)),
            });
        }

        // step 2: run the binary with race detector and concurrent load
        let run_result = self
            .docker_run_with_race_load(&source_path_str)
            .await?;

        // step 3: check for race conditions in output
        let has_race = run_result.stderr.contains("WARNING: DATA RACE")
            || run_result.stderr.contains("race detected");

        let result = match (has_race, self.expected_clean) {
            (false, true) => Ok("no race conditions detected".to_string()),
            (false, false) => Err("expected race condition but none detected".to_string()),
            (true, true) => {
                // extract race info for helpful error message
                let race_info = extract_race_info(&run_result.stderr);
                Err(format!("race condition detected:\n{}", race_info))
            }
            (true, false) => Ok("race condition detected as expected".to_string()),
        };

        Ok(TestCase {
            name: "go race detector".to_string(),
            result,
        })
    }

    /// build go code with -race flag inside docker
    async fn docker_build_with_race(&self, source_path: &str) -> Result<CommandResult, String> {
        // docker run --rm -v /path/to/source:/app -w /app golang:1.22-alpine go build -race -o /tmp/server .
        let output = Command::new("docker")
            .args([
                "run",
                "--rm",
                "-v",
                &format!("{}:/app", source_path),
                "-w",
                "/app",
                GO_IMAGE,
                "go",
                "build",
                "-race",
                "-o",
                "/tmp/server",
                ".",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("failed to run docker build: {}", e))?;

        Ok(CommandResult {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    /// run the binary with race detector and generate concurrent load
    async fn docker_run_with_race_load(&self, source_path: &str) -> Result<CommandResult, String> {
        let port = self.port;
        let requests = self.concurrent_requests;
        let timeout_duration = Duration::from_secs(self.timeout_secs);

        // script to: build with race, run server, send concurrent requests, check for races
        let test_script = format!(
            r#"
set -e

# build with race detector
cd /app
go build -race -o /tmp/server . 2>&1

# start server in background, capture stderr for race reports
/tmp/server 2>/tmp/race_output.txt &
SERVER_PID=$!

# wait for server to start
sleep 2

# check if server is running
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo "server failed to start"
    cat /tmp/race_output.txt
    exit 1
fi

# generate concurrent load
for i in $(seq 1 {requests}); do
    (
        curl -s -X POST http://localhost:{port}/jobs \
            -H "Content-Type: application/json" \
            -d '{{"type":"test","payload":"race-test-'$i'"}}' > /dev/null 2>&1 &
        curl -s http://localhost:{port}/jobs > /dev/null 2>&1 &
    ) &
done

# wait for requests to complete
wait

# give race detector time to report
sleep 2

# stop server gracefully
kill -TERM $SERVER_PID 2>/dev/null || true
sleep 1

# force kill if still running
kill -9 $SERVER_PID 2>/dev/null || true

# output race detector results
cat /tmp/race_output.txt
"#,
            requests = requests,
            port = port
        );

        // run the test script in docker with network host mode
        let result = timeout(
            timeout_duration,
            Command::new("docker")
                .args([
                    "run",
                    "--rm",
                    "--network=host",
                    "-v",
                    &format!("{}:/app", source_path),
                    GO_IMAGE,
                    "/bin/sh",
                    "-c",
                    &test_script,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => Ok(CommandResult {
                success: output.status.success(),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            }),
            Ok(Err(e)) => Err(format!("docker command failed: {}", e)),
            Err(_) => Err(format!(
                "race detector test timed out after {}s",
                self.timeout_secs
            )),
        }
    }
}

/// Result from running a command
struct CommandResult {
    success: bool,
    #[allow(dead_code)]
    stdout: String,
    stderr: String,
}

/// Check if docker is available
async fn is_docker_available() -> bool {
    Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Extract relevant race condition info from race detector output
fn extract_race_info(stderr: &str) -> String {
    let mut race_sections = Vec::new();
    let mut in_race_section = false;
    let mut current_section = Vec::new();

    for line in stderr.lines() {
        if line.contains("WARNING: DATA RACE") || line.contains("race detected") {
            in_race_section = true;
            current_section.clear();
            current_section.push(line.to_string());
        } else if in_race_section {
            // only the "======" separator ends a race section
            // empty lines are part of the race report (between "Read at" and "Previous write")
            if line.starts_with("=====") {
                if !current_section.is_empty() {
                    race_sections.push(current_section.join("\n"));
                    current_section.clear();
                }
                in_race_section = false;
            } else {
                current_section.push(line.to_string());
            }
        }
    }

    // add any remaining section
    if !current_section.is_empty() {
        race_sections.push(current_section.join("\n"));
    }

    if race_sections.is_empty() {
        // fallback: return first few lines that might be relevant
        stderr
            .lines()
            .take(20)
            .collect::<Vec<_>>()
            .join("\n")
    } else {
        // limit to first 3 race reports to avoid overwhelming output
        race_sections
            .into_iter()
            .take(3)
            .collect::<Vec<_>>()
            .join("\n\n")
    }
}

/// Validator: Compile Go code with specific flags in Docker
pub struct GoCompileValidator {
    pub source_dir: String,
    pub expected_success: bool,
    pub flags: Vec<String>,
}

impl GoCompileValidator {
    pub fn new(expected_success: bool) -> Self {
        Self {
            source_dir: ".".to_string(),
            expected_success,
            flags: vec![],
        }
    }

    pub fn with_flags(mut self, flags: Vec<String>) -> Self {
        self.flags = flags;
        self
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        if !is_docker_available().await {
            return Ok(TestCase {
                name: "go compile".to_string(),
                result: Err("docker not available".to_string()),
            });
        }

        let source_path = std::fs::canonicalize(&self.source_dir)
            .map_err(|e| format!("cannot resolve source dir: {}", e))?;

        let source_path_str = source_path.to_string_lossy();

        let mut args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "-v".to_string(),
            format!("{}:/app", source_path_str),
            "-w".to_string(),
            "/app".to_string(),
            GO_IMAGE.to_string(),
            "go".to_string(),
            "build".to_string(),
        ];

        args.extend(self.flags.clone());
        args.push(".".to_string());

        let output = Command::new("docker")
            .args(&args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("failed to run docker: {}", e))?;

        let compiled = output.status.success();
        let stderr = String::from_utf8_lossy(&output.stderr);

        let result = match (compiled, self.expected_success) {
            (true, true) => Ok("go build succeeded".to_string()),
            (false, false) => Ok("go build failed as expected".to_string()),
            (true, false) => Err("expected build to fail, but it succeeded".to_string()),
            (false, true) => {
                let preview = stderr.lines().take(10).collect::<Vec<_>>().join("\n");
                Err(format!("build failed:\n{}", preview))
            }
        };

        Ok(TestCase {
            name: "go compile".to_string(),
            result,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ==========================================
    // Unit tests for helper functions
    // ==========================================

    #[test]
    fn test_extract_race_info_empty() {
        let info = extract_race_info("");
        assert!(info.is_empty());
    }

    #[test]
    fn test_extract_race_info_with_single_race() {
        let stderr = r#"
some output
WARNING: DATA RACE
Read at 0x00c0000a4008 by goroutine 7:
  main.handler()
      /app/main.go:42 +0x64

Previous write at 0x00c0000a4008 by goroutine 8:
  main.worker()
      /app/main.go:58 +0x78
==================
more output
"#;
        let info = extract_race_info(stderr);
        assert!(info.contains("WARNING: DATA RACE"));
        assert!(info.contains("main.handler"));
        assert!(info.contains("main.worker"));
    }

    #[test]
    fn test_extract_race_info_with_multiple_races() {
        let stderr = r#"
==================
WARNING: DATA RACE
Read at 0x00c0000a4008 by goroutine 7:
  main.handleGet()
      /app/main.go:42 +0x64
==================
WARNING: DATA RACE
Write at 0x00c0000a4010 by goroutine 8:
  main.handlePost()
      /app/main.go:58 +0x78
==================
WARNING: DATA RACE
Read at 0x00c0000a4020 by goroutine 9:
  main.handleList()
      /app/main.go:74 +0x90
==================
WARNING: DATA RACE
Write at 0x00c0000a4030 by goroutine 10:
  main.handleDelete()
      /app/main.go:90 +0xa0
==================
"#;
        let info = extract_race_info(stderr);
        // should only show first 3 races
        assert!(info.contains("handleGet"));
        assert!(info.contains("handlePost"));
        assert!(info.contains("handleList"));
        // fourth race should be truncated
        assert!(!info.contains("handleDelete"));
    }

    #[test]
    fn test_extract_race_info_with_race_detected_keyword() {
        let stderr = "race detected during execution\nsome details here";
        let info = extract_race_info(stderr);
        assert!(info.contains("race detected"));
    }

    #[test]
    fn test_extract_race_info_no_race_fallback() {
        let stderr = "line 1\nline 2\nline 3\nline 4\nline 5";
        let info = extract_race_info(stderr);
        // should return first lines as fallback
        assert!(info.contains("line 1"));
    }

    // ==========================================
    // RaceDetectorValidator unit tests
    // ==========================================

    #[test]
    fn test_race_detector_validator_new() {
        let validator = RaceDetectorValidator::new(true);
        assert!(validator.expected_clean);
        assert_eq!(validator.source_dir, ".");
        assert_eq!(validator.timeout_secs, DEFAULT_TIMEOUT_SECS);
        assert_eq!(validator.concurrent_requests, 50);
        assert_eq!(validator.port, 8080);
    }

    #[test]
    fn test_race_detector_validator_new_expects_race() {
        let validator = RaceDetectorValidator::new(false);
        assert!(!validator.expected_clean);
    }

    #[test]
    fn test_race_detector_validator_with_source_dir() {
        let validator = RaceDetectorValidator::new(true).with_source_dir("/path/to/project");
        assert_eq!(validator.source_dir, "/path/to/project");
    }

    #[test]
    fn test_race_detector_validator_with_timeout() {
        let validator = RaceDetectorValidator::new(true).with_timeout(60);
        assert_eq!(validator.timeout_secs, 60);
    }

    #[test]
    fn test_race_detector_validator_builder_chain() {
        let validator = RaceDetectorValidator::new(true)
            .with_source_dir("/app")
            .with_timeout(30);

        assert!(validator.expected_clean);
        assert_eq!(validator.source_dir, "/app");
        assert_eq!(validator.timeout_secs, 30);
    }

    // ==========================================
    // GoCompileValidator unit tests
    // ==========================================

    #[test]
    fn test_go_compile_validator_new() {
        let validator = GoCompileValidator::new(true);
        assert!(validator.expected_success);
        assert_eq!(validator.source_dir, ".");
        assert!(validator.flags.is_empty());
    }

    #[test]
    fn test_go_compile_validator_expects_failure() {
        let validator = GoCompileValidator::new(false);
        assert!(!validator.expected_success);
    }

    #[test]
    fn test_go_compile_validator_with_flags() {
        let validator =
            GoCompileValidator::new(true).with_flags(vec!["-race".to_string(), "-v".to_string()]);
        assert_eq!(validator.flags, vec!["-race", "-v"]);
    }

    // ==========================================
    // CommandResult tests
    // ==========================================

    #[test]
    fn test_command_result_success() {
        let result = CommandResult {
            success: true,
            stdout: "build successful".to_string(),
            stderr: String::new(),
        };
        assert!(result.success);
        assert!(result.stderr.is_empty());
    }

    #[test]
    fn test_command_result_failure() {
        let result = CommandResult {
            success: false,
            stdout: String::new(),
            stderr: "compilation error".to_string(),
        };
        assert!(!result.success);
        assert!(result.stderr.contains("compilation error"));
    }

    // ==========================================
    // Async tests (require tokio runtime)
    // ==========================================

    #[tokio::test]
    async fn test_is_docker_available_returns_bool() {
        // this test just verifies the function runs without panic
        // actual result depends on whether docker is installed
        let _available = is_docker_available().await;
    }

    #[tokio::test]
    async fn test_race_detector_validate_invalid_source_dir() {
        let validator =
            RaceDetectorValidator::new(true).with_source_dir("/nonexistent/path/12345");

        let result = validator.validate().await;

        // should return error about source directory
        assert!(result.is_err() || {
            let test_case = result.unwrap();
            test_case.result.is_err()
        });
    }

    #[tokio::test]
    async fn test_go_compile_validator_invalid_source_dir() {
        let validator = GoCompileValidator {
            source_dir: "/nonexistent/path/12345".to_string(),
            expected_success: true,
            flags: vec![],
        };

        let result = validator.validate().await;

        // should return error about source directory or docker
        assert!(result.is_err() || {
            let test_case = result.unwrap();
            test_case.result.is_err()
        });
    }

    // ==========================================
    // Factory integration tests
    // ==========================================

    #[test]
    fn test_create_race_detector_from_string() {
        use crate::validators::factory::create_validator;

        let validator = create_validator("race_detector:bool(true)").unwrap();
        assert_eq!(validator.name(), "race_detector");
    }

    #[test]
    fn test_create_race_detector_expects_race() {
        use crate::validators::factory::create_validator;

        let validator = create_validator("race_detector:bool(false)").unwrap();
        assert_eq!(validator.name(), "race_detector");
    }

    #[test]
    fn test_create_go_compile_from_string() {
        use crate::validators::factory::create_validator;

        let validator = create_validator("go_compile:bool(true)").unwrap();
        assert_eq!(validator.name(), "go_compile");
    }

    #[test]
    fn test_create_go_compile_expects_failure() {
        use crate::validators::factory::create_validator;

        let validator = create_validator("go_compile:bool(false)").unwrap();
        assert_eq!(validator.name(), "go_compile");
    }
}

// ==========================================
// Integration tests (require Docker)
// These tests are ignored by default since they need Docker
// Run with: cargo test -- --ignored
// ==========================================

#[cfg(test)]
mod integration_tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Helper to create a temporary Go project for testing
    fn create_test_go_project(code: &str) -> TempDir {
        let temp_dir = TempDir::new().expect("failed to create temp dir");

        // write go.mod
        fs::write(
            temp_dir.path().join("go.mod"),
            "module testproject\n\ngo 1.22\n",
        )
        .expect("failed to write go.mod");

        // write main.go
        fs::write(temp_dir.path().join("main.go"), code).expect("failed to write main.go");

        temp_dir
    }

    #[tokio::test]
    #[ignore] // requires docker
    async fn test_race_detector_with_clean_code() {
        let code = r#"
package main

import (
    "fmt"
    "net/http"
    "sync"
)

var (
    counter int
    mu      sync.Mutex
)

func handler(w http.ResponseWriter, r *http.Request) {
    mu.Lock()
    counter++
    mu.Unlock()
    fmt.Fprintf(w, "count: %d", counter)
}

func main() {
    http.HandleFunc("/", handler)
    http.ListenAndServe(":8080", nil)
}
"#;

        let temp_dir = create_test_go_project(code);
        let validator = RaceDetectorValidator::new(true)
            .with_source_dir(temp_dir.path().to_str().unwrap())
            .with_timeout(60);

        let result = validator.validate().await;
        assert!(result.is_ok());

        let test_case = result.unwrap();
        // clean code should pass
        assert!(
            test_case.result.is_ok(),
            "expected clean code to pass: {:?}",
            test_case.result
        );
    }

    #[tokio::test]
    #[ignore] // requires docker
    async fn test_race_detector_with_racy_code() {
        let code = r#"
package main

import (
    "fmt"
    "net/http"
)

var counter int // no mutex protection - will cause race

func handler(w http.ResponseWriter, r *http.Request) {
    counter++ // race condition here
    fmt.Fprintf(w, "count: %d", counter)
}

func main() {
    http.HandleFunc("/", handler)
    http.ListenAndServe(":8080", nil)
}
"#;

        let temp_dir = create_test_go_project(code);
        let validator = RaceDetectorValidator::new(true)
            .with_source_dir(temp_dir.path().to_str().unwrap())
            .with_timeout(60);

        let result = validator.validate().await;
        assert!(result.is_ok());

        let test_case = result.unwrap();
        // racy code should fail when expecting clean
        assert!(
            test_case.result.is_err(),
            "expected racy code to fail: {:?}",
            test_case.result
        );
    }

    #[tokio::test]
    #[ignore] // requires docker
    async fn test_race_detector_expects_race_with_racy_code() {
        let code = r#"
package main

import (
    "fmt"
    "net/http"
)

var counter int

func handler(w http.ResponseWriter, r *http.Request) {
    counter++
    fmt.Fprintf(w, "count: %d", counter)
}

func main() {
    http.HandleFunc("/", handler)
    http.ListenAndServe(":8080", nil)
}
"#;

        let temp_dir = create_test_go_project(code);
        let validator = RaceDetectorValidator::new(false) // expect race
            .with_source_dir(temp_dir.path().to_str().unwrap())
            .with_timeout(60);

        let result = validator.validate().await;
        assert!(result.is_ok());

        let test_case = result.unwrap();
        // when expecting race, racy code should pass
        assert!(
            test_case.result.is_ok(),
            "expected to detect race: {:?}",
            test_case.result
        );
    }

    #[tokio::test]
    #[ignore] // requires docker
    async fn test_go_compile_valid_code() {
        let code = r#"
package main

import "fmt"

func main() {
    fmt.Println("hello")
}
"#;

        let temp_dir = create_test_go_project(code);
        let validator = GoCompileValidator {
            source_dir: temp_dir.path().to_str().unwrap().to_string(),
            expected_success: true,
            flags: vec![],
        };

        let result = validator.validate().await;
        assert!(result.is_ok());

        let test_case = result.unwrap();
        assert!(
            test_case.result.is_ok(),
            "expected valid code to compile: {:?}",
            test_case.result
        );
    }

    #[tokio::test]
    #[ignore] // requires docker
    async fn test_go_compile_invalid_code() {
        let code = r#"
package main

func main() {
    undefined_function() // this will fail
}
"#;

        let temp_dir = create_test_go_project(code);
        let validator = GoCompileValidator {
            source_dir: temp_dir.path().to_str().unwrap().to_string(),
            expected_success: true,
            flags: vec![],
        };

        let result = validator.validate().await;
        assert!(result.is_ok());

        let test_case = result.unwrap();
        assert!(
            test_case.result.is_err(),
            "expected invalid code to fail: {:?}",
            test_case.result
        );
    }

    #[tokio::test]
    #[ignore] // requires docker
    async fn test_go_compile_with_race_flag() {
        let code = r#"
package main

import "fmt"

func main() {
    fmt.Println("hello")
}
"#;

        let temp_dir = create_test_go_project(code);
        let validator = GoCompileValidator {
            source_dir: temp_dir.path().to_str().unwrap().to_string(),
            expected_success: true,
            flags: vec!["-race".to_string()],
        };

        let result = validator.validate().await;
        assert!(result.is_ok());

        let test_case = result.unwrap();
        assert!(
            test_case.result.is_ok(),
            "expected code to compile with -race: {:?}",
            test_case.result
        );
    }
}
