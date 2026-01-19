//! Shell command execution for prologue/epilogue hooks

use std::process::Stdio;
use tokio::process::Command;

/// result of running a shell command
#[derive(Debug)]
pub struct CommandResult {
    pub exit_code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl CommandResult {
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

/// run a shell command and capture output
pub async fn run_command(cmd: &str) -> Result<CommandResult, String> {
    log::debug!("running command: {}", cmd);

    let output = Command::new("sh")
        .args(["-c", cmd])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
        .map_err(|e| format!("failed to execute command: {}", e))?;

    let result = CommandResult {
        exit_code: output.status.code().unwrap_or(-1),
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
    };

    log::debug!(
        "command finished with exit code {}: stdout={} bytes, stderr={} bytes",
        result.exit_code,
        result.stdout.len(),
        result.stderr.len()
    );

    Ok(result)
}

/// run a list of commands sequentially, stopping on first failure
/// returns Ok(()) if all commands succeed, Err with the failing command on failure
pub async fn run_commands(commands: &[String]) -> Result<(), (String, CommandResult)> {
    for cmd in commands {
        let result = run_command(cmd).await.map_err(|e| {
            (
                cmd.clone(),
                CommandResult {
                    exit_code: -1,
                    stdout: String::new(),
                    stderr: e,
                },
            )
        })?;

        if !result.success() {
            return Err((cmd.clone(), result));
        }
    }
    Ok(())
}

/// run a list of commands, continuing even on failure (for cleanup)
/// returns a list of (command, result) for any failed commands
pub async fn run_commands_best_effort(commands: &[String]) -> Vec<(String, CommandResult)> {
    let mut failures = Vec::new();

    for cmd in commands {
        match run_command(cmd).await {
            Ok(result) if !result.success() => {
                failures.push((cmd.clone(), result));
            }
            Err(e) => {
                failures.push((
                    cmd.clone(),
                    CommandResult {
                        exit_code: -1,
                        stdout: String::new(),
                        stderr: e,
                    },
                ));
            }
            _ => {}
        }
    }

    failures
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_command_success() {
        let result = run_command("echo hello").await.unwrap();
        assert!(result.success());
        assert_eq!(result.stdout.trim(), "hello");
    }

    #[tokio::test]
    async fn test_run_command_failure() {
        let result = run_command("exit 1").await.unwrap();
        assert!(!result.success());
        assert_eq!(result.exit_code, 1);
    }

    #[tokio::test]
    async fn test_run_commands_all_succeed() {
        let commands = vec!["echo one".to_string(), "echo two".to_string()];
        let result = run_commands(&commands).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_run_commands_stops_on_failure() {
        let commands = vec![
            "echo one".to_string(),
            "exit 1".to_string(),
            "echo three".to_string(),
        ];
        let result = run_commands(&commands).await;
        assert!(result.is_err());
        let (cmd, _) = result.unwrap_err();
        assert_eq!(cmd, "exit 1");
    }

    #[tokio::test]
    async fn test_run_commands_best_effort_continues() {
        let commands = vec![
            "echo one".to_string(),
            "exit 1".to_string(),
            "exit 2".to_string(),
        ];
        let failures = run_commands_best_effort(&commands).await;
        assert_eq!(failures.len(), 2);
    }
}
