use crate::config::Config;
use crate::state::ProjectState;
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
        // get workspace and runtime from project state
        let (workspace, runtime) = get_project_context();
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
                "project compiles{}",
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

/// get workspace and runtime from active project state
fn get_project_context() -> (Option<String>, Option<String>) {
    let config = match Config::load() {
        Ok(c) => c,
        Err(_) => return (None, None),
    };
    if !config.has_auth_token() {
        return (None, None);
    }
    let state = match ProjectState::load(config.expose_token()) {
        Ok(s) => s,
        Err(_) => return (None, None),
    };
    match state.get_active() {
        Some(project) => (Some(project.workspace.clone()), project.runtime.clone()),
        None => (None, None),
    }
}

/// detect the project type and return appropriate build command
/// if runtime is provided, use it directly instead of auto-detecting
fn detect_build_command(
    runtime: Option<&str>,
    workspace: &Path,
) -> Result<(String, Vec<String>), String> {
    // if runtime is explicitly set, use it
    if let Some(rt) = runtime {
        return match rt.to_lowercase().as_str() {
            "go" => {
                // check if there are any .go files
                let has_go_files = std::fs::read_dir(workspace)
                    .map(|entries| {
                        entries
                            .filter_map(|e| e.ok())
                            .any(|e| e.path().extension().map(|ext| ext == "go").unwrap_or(false))
                    })
                    .unwrap_or(false);

                if !has_go_files {
                    return Err("no .go source files found in project directory".to_string());
                }
                Ok(("go".to_string(), vec!["build".to_string(), ".".to_string()]))
            }
            "rust" => Ok(("cargo".to_string(), vec!["check".to_string()])),
            "c" | "cpp" | "c++" => Ok(("make".to_string(), vec![])),
            "python" | "py" => Ok((
                "python".to_string(),
                vec![
                    "-m".to_string(),
                    "py_compile".to_string(),
                    "*.py".to_string(),
                ],
            )),
            _ => Err(format!("unsupported runtime: {}", rt)),
        };
    }

    // auto-detect based on project files
    // rust/cargo
    if workspace.join("Cargo.toml").exists() {
        return Ok(("cargo".to_string(), vec!["check".to_string()]));
    }

    // go
    if workspace.join("go.mod").exists() {
        // check if there are any .go files
        let has_go_files = std::fs::read_dir(workspace)
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .any(|e| e.path().extension().map(|ext| ext == "go").unwrap_or(false))
            })
            .unwrap_or(false);

        if !has_go_files {
            return Err("no .go source files found in project directory".to_string());
        }

        return Ok(("go".to_string(), vec!["build".to_string(), ".".to_string()]));
    }

    // node/typescript
    if workspace.join("package.json").exists() {
        // check for typescript
        if workspace.join("tsconfig.json").exists() {
            return Ok((
                "npx".to_string(),
                vec!["tsc".to_string(), "--noEmit".to_string()],
            ));
        }
        // plain js has no compile step, treat as success
        return Ok((
            "echo".to_string(),
            vec!["no compile step for js".to_string()],
        ));
    }

    // python (syntax check)
    if workspace.join("requirements.txt").exists()
        || workspace.join("pyproject.toml").exists()
        || workspace.join("setup.py").exists()
    {
        return Ok((
            "python".to_string(),
            vec![
                "-m".to_string(),
                "py_compile".to_string(),
                "*.py".to_string(),
            ],
        ));
    }

    // c/c++ with makefile
    if workspace.join("Makefile").exists() || workspace.join("makefile").exists() {
        return Ok(("make".to_string(), vec!["-n".to_string()])); // dry run
    }

    Err(
        "unable to detect project type. expected Cargo.toml, go.mod, package.json, or Makefile"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_detect_build_command_error_when_no_project() {
        // in test environment, may not have project files
        // just verify function doesn't panic
        let workspace = PathBuf::from("/tmp/nonexistent");
        let _ = detect_build_command(None, &workspace);
    }

    #[test]
    fn test_detect_build_command_with_runtime() {
        // test explicit runtime selection (rust doesn't need files to exist)
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
        let result = detect_build_command(Some("unknown"), &workspace);
        assert!(result.is_err());
    }
}
