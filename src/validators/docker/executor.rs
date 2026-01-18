//! Docker executor - downloads Dockerfiles and runs containers

use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use tokio::time::{timeout, Duration};

const DOCKERFILE_BASE_URL: &str =
    "https://raw.githubusercontent.com/thearyanahmed/luxctl/master/docker";
const DEFAULT_TIMEOUT_SECS: u64 = 120;

/// result from running a container
#[derive(Debug)]
pub struct ExecutorResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl ExecutorResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// executor for running Dockerfiles
pub struct DockerExecutor {
    cache_dir: PathBuf,
}

impl DockerExecutor {
    pub fn new() -> Result<Self, String> {
        let cache_dir = dirs::home_dir()
            .ok_or("could not determine home directory")?
            .join(".luxctl")
            .join("docker_cache");

        std::fs::create_dir_all(&cache_dir)
            .map_err(|e| format!("failed to create cache dir: {}", e))?;

        Ok(Self { cache_dir })
    }

    /// download a Dockerfile by name from GitHub
    pub async fn download_dockerfile(&self, name: &str) -> Result<PathBuf, String> {
        let url = format!("{}/{}", DOCKERFILE_BASE_URL, name);
        let cache_path = self.cache_dir.join(name);

        // fetch from GitHub
        let response = reqwest::get(&url)
            .await
            .map_err(|e| format!("failed to fetch Dockerfile '{}': {}", name, e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Dockerfile '{}' not found (status {})",
                name,
                response.status()
            ));
        }

        let content = response
            .text()
            .await
            .map_err(|e| format!("failed to read Dockerfile content: {}", e))?;

        // cache locally
        std::fs::write(&cache_path, &content)
            .map_err(|e| format!("failed to cache Dockerfile: {}", e))?;

        Ok(cache_path)
    }

    /// build and run a container from a Dockerfile
    pub async fn run(
        &self,
        dockerfile_name: &str,
        workspace: &str,
        timeout_secs: Option<u64>,
    ) -> Result<ExecutorResult, String> {
        // check docker availability
        if !is_docker_available().await {
            return Err("docker not available".to_string());
        }

        // download Dockerfile
        let dockerfile_path = self.download_dockerfile(dockerfile_name).await?;

        // resolve workspace to absolute path
        let workspace_path = std::fs::canonicalize(workspace)
            .map_err(|e| format!("cannot resolve workspace '{}': {}", workspace, e))?;

        let workspace_str = workspace_path.to_string_lossy();

        // generate unique image tag
        let image_tag = format!(
            "luxctl-{}:{}",
            dockerfile_name.to_lowercase().replace('.', "-"),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0)
        );

        // build the image
        eprintln!("  building {} (this may take a moment)...", dockerfile_name);
        let build_result = self
            .docker_build(&dockerfile_path, &workspace_str, &image_tag)
            .await?;

        if !build_result.success() {
            return Ok(build_result);
        }

        // run the container
        eprintln!("  running validation...");
        let run_result = self
            .docker_run(&image_tag, &workspace_str, timeout_secs)
            .await;

        // cleanup: remove the image
        let _ = Command::new("docker")
            .args(["rmi", "-f", &image_tag])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .await;

        run_result
    }

    async fn docker_build(
        &self,
        dockerfile_path: &PathBuf,
        context: &str,
        tag: &str,
    ) -> Result<ExecutorResult, String> {
        let output = Command::new("docker")
            .args([
                "build",
                "-f",
                dockerfile_path.to_string_lossy().as_ref(),
                "-t",
                tag,
                context,
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("failed to run docker build: {}", e))?;

        Ok(ExecutorResult {
            exit_code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        })
    }

    async fn docker_run(
        &self,
        image: &str,
        workspace: &str,
        timeout_secs: Option<u64>,
    ) -> Result<ExecutorResult, String> {
        let timeout_duration = Duration::from_secs(timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS));

        let result = timeout(
            timeout_duration,
            Command::new("docker")
                .args([
                    "run",
                    "--rm",
                    "--network=host",
                    "-v",
                    &format!("{}:/app", workspace),
                    "-w",
                    "/app",
                    image,
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .output(),
        )
        .await;

        match result {
            Ok(Ok(output)) => Ok(ExecutorResult {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).to_string(),
                stderr: String::from_utf8_lossy(&output.stderr).to_string(),
            }),
            Ok(Err(e)) => Err(format!("docker run failed: {}", e)),
            Err(_) => Err(format!(
                "container timed out after {}s",
                timeout_secs.unwrap_or(DEFAULT_TIMEOUT_SECS)
            )),
        }
    }
}

impl Default for DockerExecutor {
    fn default() -> Self {
        Self::new().expect("failed to create DockerExecutor")
    }
}

/// check if docker is available
pub async fn is_docker_available() -> bool {
    Command::new("docker")
        .args(["version", "--format", "{{.Server.Version}}"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_executor_result_success() {
        let result = ExecutorResult {
            exit_code: 0,
            stdout: "ok".to_string(),
            stderr: String::new(),
        };
        assert!(result.success());
    }

    #[test]
    fn test_executor_result_failure() {
        let result = ExecutorResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
        };
        assert!(!result.success());
    }

    #[tokio::test]
    async fn test_is_docker_available_returns_bool() {
        // just verify it doesn't panic
        let _ = is_docker_available().await;
    }
}
