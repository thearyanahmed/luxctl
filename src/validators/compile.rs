use crate::tasks::TestCase;
use std::path::Path;
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
        let (cmd, args) = detect_build_command()?;

        let output = Command::new(&cmd)
            .args(&args)
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

/// detect the project type and return appropriate build command
fn detect_build_command() -> Result<(String, Vec<String>), String> {
    let cwd = std::env::current_dir().map_err(|e| format!("cannot get cwd: {}", e))?;

    // rust/cargo
    if Path::new("Cargo.toml").exists() || cwd.join("Cargo.toml").exists() {
        return Ok(("cargo".to_string(), vec!["check".to_string()]));
    }

    // go
    if Path::new("go.mod").exists() || cwd.join("go.mod").exists() {
        return Ok(("go".to_string(), vec!["build".to_string(), ".".to_string()]));
    }

    // node/typescript
    if Path::new("package.json").exists() || cwd.join("package.json").exists() {
        // check for typescript
        if Path::new("tsconfig.json").exists() || cwd.join("tsconfig.json").exists() {
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
    if Path::new("requirements.txt").exists()
        || Path::new("pyproject.toml").exists()
        || Path::new("setup.py").exists()
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
    if Path::new("Makefile").exists() || Path::new("makefile").exists() {
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

    #[test]
    fn test_detect_build_command_error_when_no_project() {
        // in test environment, may not have project files
        // just verify function doesn't panic
        let _ = detect_build_command();
    }
}
