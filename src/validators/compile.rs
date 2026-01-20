use crate::config::Config;
use crate::runtime::SupportedRuntime;
use crate::state::LabState;
use crate::tasks::TestCase;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::process::Command;

/// Validator: check if project compiles successfully
pub struct CanCompileValidator {
    pub expected_success: bool,
}

impl CanCompileValidator {
    pub fn new(expected_success: bool) -> Self {
        Self { expected_success }
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        // get workspace and runtime from lab state
        let (workspace, runtime) = get_lab_context();
        let workspace_path = workspace
            .map(PathBuf::from)
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

        let (cmd, args) = detect_build_command(runtime.as_deref(), &workspace_path)?;

        let output = Command::new(&cmd)
            .args(&args)
            .current_dir(&workspace_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| format!("failed to run '{}': {}", cmd, e))?;

        let compiled_ok = output.status.success();
        let stderr = String::from_utf8_lossy(&output.stderr);

        let result = match (compiled_ok, self.expected_success) {
            (true, true) => Ok(format!("{} {} succeeded", cmd, args.join(" "))),
            (false, false) => Ok("compilation failed as expected".to_string()),
            (true, false) => Err("expected compilation to fail, but it succeeded".to_string()),
            (false, true) => {
                let err_preview = stderr.lines().take(5).collect::<Vec<_>>().join("\n");
                Err(format!("compilation failed:\n{}", err_preview))
            }
        };

        Ok(TestCase {
            name: format!(
                "lab compiles{}",
                if self.expected_success {
                    ""
                } else {
                    " (expected failure)"
                }
            ),
            result,
        })
    }
}

/// get workspace and runtime from active lab state
fn get_lab_context() -> (Option<String>, Option<String>) {
    let config = match Config::load() {
        Ok(c) => c,
        Err(_) => return (None, None),
    };
    if !config.has_auth_token() {
        return (None, None);
    }
    let state = match LabState::load(config.expose_token()) {
        Ok(s) => s,
        Err(_) => return (None, None),
    };
    match state.get_active() {
        Some(lab) => (Some(lab.workspace.clone()), lab.runtime.clone()),
        None => (None, None),
    }
}

/// detect the project type and return appropriate build command
/// if runtime is provided, use it directly instead of auto-detecting
fn detect_build_command(
    runtime: Option<&str>,
    workspace: &Path,
) -> Result<(String, Vec<String>), String> {
    // if runtime is explicitly set, parse and use it
    if let Some(rt) = runtime {
        let supported: SupportedRuntime = rt.parse()?;
        return get_build_command_for_runtime(supported, workspace);
    }

    // auto-detect based on project files
    let detected = SupportedRuntime::detect(workspace).ok_or_else(|| {
        format!(
            "unable to detect project type. expected {} in workspace",
            SupportedRuntime::all()
                .iter()
                .map(|r| r.module_file())
                .collect::<Vec<_>>()
                .join(" or ")
        )
    })?;

    get_build_command_for_runtime(detected, workspace)
}

/// get build command for a specific runtime, validating source files exist
fn get_build_command_for_runtime(
    runtime: SupportedRuntime,
    workspace: &Path,
) -> Result<(String, Vec<String>), String> {
    // for Go, verify source files exist (Rust's cargo check handles this)
    if runtime == SupportedRuntime::Go && !runtime.has_source_files(workspace) {
        return Err(format!(
            "no .{} source files found in project directory",
            runtime.extension()
        ));
    }

    Ok((
        runtime.build_command().to_string(),
        runtime.build_args().iter().map(|s| s.to_string()).collect(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_build_command_error_when_no_project() {
        let workspace = PathBuf::from("/tmp/nonexistent");
        let result = detect_build_command(None, &workspace);
        assert!(result.is_err());
    }

    #[test]
    fn test_detect_build_command_with_runtime() {
        // rust doesn't need files to exist for cargo check
        let workspace = PathBuf::from("/tmp");
        let result = detect_build_command(Some("rust"), &workspace);
        assert!(result.is_ok());
        let (cmd, args) = result.unwrap();
        assert_eq!(cmd, "cargo");
        assert_eq!(args, vec!["check"]);
    }

    #[test]
    fn test_detect_build_command_unsupported_runtime() {
        let workspace = PathBuf::from("/tmp");
        let result = detect_build_command(Some("python"), &workspace);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("unsupported runtime"));
    }
}
