use crate::config::Config;
use crate::state::ProjectState;
use crate::tasks::TestCase;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

const DEFAULT_TIMEOUT_MS: u64 = 5000;

/// get workspace from active project state
fn get_workspace() -> Option<PathBuf> {
    let config = Config::load().ok()?;
    if !config.has_auth_token() {
        return None;
    }
    let state = ProjectState::load(config.expose_token()).ok()?;
    state.get_active().map(|p| PathBuf::from(&p.workspace))
}

/// Validator: test graceful shutdown behavior
/// starts a process, sends SIGTERM, verifies it exits cleanly
pub struct GracefulShutdownValidator {
    pub binary_path: String,
    pub timeout_ms: u64,
    pub expected_exit_code: i32,
    pub startup_wait_ms: u64,
}

impl GracefulShutdownValidator {
    pub fn new(binary_path: &str, timeout_ms: u64) -> Self {
        Self {
            binary_path: binary_path.to_string(),
            timeout_ms,
            expected_exit_code: 0,
            startup_wait_ms: 1000,
        }
    }

    pub fn with_expected_exit_code(mut self, code: i32) -> Self {
        self.expected_exit_code = code;
        self
    }

    pub fn with_startup_wait(mut self, ms: u64) -> Self {
        self.startup_wait_ms = ms;
        self
    }

    #[cfg(unix)]
    pub async fn validate(&self) -> Result<TestCase, String> {
        use nix::sys::signal::{kill, Signal};
        use nix::unistd::Pid;

        // get workspace path from project state
        let workspace = get_workspace()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        // spawn the process from the workspace directory
        let mut child = Command::new(&self.binary_path)
            .current_dir(&workspace)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("failed to spawn process: {}", e))?;

        let pid = child.id().ok_or("failed to get process id")?;

        // wait for process to start
        tokio::time::sleep(Duration::from_millis(self.startup_wait_ms)).await;

        // send SIGTERM
        let nix_pid = Pid::from_raw(pid as i32);
        kill(nix_pid, Signal::SIGTERM).map_err(|e| format!("failed to send SIGTERM: {}", e))?;

        // wait for graceful exit with timeout
        let shutdown_timeout = Duration::from_millis(self.timeout_ms);
        let wait_result = timeout(shutdown_timeout, child.wait()).await;

        let result = match wait_result {
            Ok(Ok(status)) => {
                let exit_code = status.code().unwrap_or(-1);
                if exit_code == self.expected_exit_code {
                    Ok(format!(
                        "process exited gracefully with code {} after SIGTERM",
                        exit_code
                    ))
                } else {
                    Err(format!(
                        "expected exit code {}, got {}",
                        self.expected_exit_code, exit_code
                    ))
                }
            }
            Ok(Err(e)) => Err(format!("failed to wait for process: {}", e)),
            Err(_) => {
                // timeout - process didn't exit gracefully, kill it
                let _ = child.kill().await;
                Err(format!(
                    "process did not exit within {}ms after SIGTERM",
                    self.timeout_ms
                ))
            }
        };

        Ok(TestCase {
            name: format!("graceful shutdown within {}ms", self.timeout_ms),
            result,
        })
    }

    #[cfg(not(unix))]
    pub async fn validate(&self) -> Result<TestCase, String> {
        Ok(TestCase {
            name: "graceful shutdown".to_string(),
            result: Err("graceful_shutdown validator only supported on Unix systems".to_string()),
        })
    }
}

/// Validator: check if a process handles concurrent requests safely
/// spawns multiple concurrent operations and checks for data races or deadlocks
pub struct ConcurrentAccessValidator {
    pub port: u16,
    pub path: String,
    pub concurrent_count: u32,
    pub operations_per_client: u32,
    pub timeout_ms: u64,
}

impl ConcurrentAccessValidator {
    pub fn new(port: u16, path: &str, concurrent_count: u32, operations_per_client: u32) -> Self {
        Self {
            port,
            path: path.to_string(),
            concurrent_count,
            operations_per_client,
            timeout_ms: DEFAULT_TIMEOUT_MS,
        }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        use super::http::http_request;

        let mut handles = Vec::new();

        for client_id in 0..self.concurrent_count {
            let port = self.port;
            let path = self.path.clone();
            let ops = self.operations_per_client;

            let handle = tokio::spawn(async move {
                let mut results = Vec::new();
                for op_id in 0..ops {
                    let response = http_request(port, "GET", &path, &[], None).await;
                    results.push((client_id, op_id, response));
                }
                results
            });
            handles.push(handle);
        }

        let timeout_duration = Duration::from_millis(self.timeout_ms);
        let all_results = timeout(timeout_duration, async {
            let mut all = Vec::new();
            for handle in handles {
                match handle.await {
                    Ok(results) => all.extend(results),
                    Err(e) => return Err(format!("task panicked: {}", e)),
                }
            }
            Ok(all)
        })
        .await;

        let result = match all_results {
            Ok(Ok(results)) => {
                let total = results.len();
                let successes = results.iter().filter(|(_, _, r)| r.is_ok()).count();
                let failures: Vec<_> = results
                    .iter()
                    .filter_map(|(c, o, r)| {
                        r.as_ref()
                            .err()
                            .map(|e| format!("client {}, op {}: {}", c, o, e))
                    })
                    .collect();

                if failures.is_empty() {
                    Ok(format!(
                        "all {}/{} concurrent operations completed successfully",
                        successes, total
                    ))
                } else {
                    // limit error output to first 3 failures
                    let error_summary = if failures.len() <= 3 {
                        failures.join("; ")
                    } else {
                        format!(
                            "{}; ... and {} more failures",
                            failures[..3].join("; "),
                            failures.len() - 3
                        )
                    };
                    Err(format!(
                        "{}/{} operations failed: {}",
                        failures.len(),
                        total,
                        error_summary
                    ))
                }
            }
            Ok(Err(e)) => Err(e),
            Err(_) => Err(format!(
                "concurrent operations timed out after {}ms - possible deadlock",
                self.timeout_ms
            )),
        };

        Ok(TestCase {
            name: format!(
                "{} concurrent clients x {} operations",
                self.concurrent_count, self.operations_per_client
            ),
            result,
        })
    }
}
