//! Docker validator - runs Dockerfiles and interprets results based on DSL

use crate::config::Config;
use crate::state::LabState;
use crate::tasks::TestCase;

use super::executor::DockerExecutor;
use std::path::PathBuf;

/// expectation for validating container output
#[derive(Debug, Clone)]
pub enum Expectation {
    /// expect specific exit code
    ExitCode(i32),
    /// fail if stdout contains string
    FailIfStdoutContains(String),
    /// fail if stderr contains string
    FailIfStderrContains(String),
    /// pass if stdout contains string
    PassIfStdoutContains(String),
    /// pass if stderr contains string
    PassIfStderrContains(String),
}

impl Expectation {
    /// parse expectation from DSL string
    /// formats:
    ///   "exit:0" - expect exit code 0
    ///   "fail_if:stdout contains X" - fail if stdout contains X
    ///   "fail_if:stderr contains X" - fail if stderr contains X
    ///   "pass_if:stdout contains X" - pass if stdout contains X
    ///   "pass_if:stderr contains X" - pass if stderr contains X
    pub fn parse(s: &str) -> Result<Self, String> {
        let s = s.trim();

        if let Some(code) = s.strip_prefix("exit:") {
            let code: i32 = code
                .trim()
                .parse()
                .map_err(|_| format!("invalid exit code: {}", code))?;
            return Ok(Expectation::ExitCode(code));
        }

        if let Some(rest) = s.strip_prefix("fail_if:") {
            return parse_contains_expectation(rest, true);
        }

        if let Some(rest) = s.strip_prefix("pass_if:") {
            return parse_contains_expectation(rest, false);
        }

        Err(format!("unknown expectation format: {}", s))
    }
}

fn parse_contains_expectation(s: &str, is_fail_if: bool) -> Result<Expectation, String> {
    let s = s.trim();

    if let Some(rest) = s.strip_prefix("stdout contains ") {
        let pattern = rest.trim().to_string();
        return Ok(if is_fail_if {
            Expectation::FailIfStdoutContains(pattern)
        } else {
            Expectation::PassIfStdoutContains(pattern)
        });
    }

    if let Some(rest) = s.strip_prefix("stderr contains ") {
        let pattern = rest.trim().to_string();
        return Ok(if is_fail_if {
            Expectation::FailIfStderrContains(pattern)
        } else {
            Expectation::PassIfStderrContains(pattern)
        });
    }

    Err(format!(
        "invalid contains format, expected 'stdout contains X' or 'stderr contains X': {}",
        s
    ))
}

/// validator that runs a Dockerfile and checks expectations
pub struct DockerValidator {
    pub dockerfile_name: String,
    pub expectation: Expectation,
    pub timeout_secs: Option<u64>,
}

impl DockerValidator {
    pub fn new(dockerfile_name: &str, expectation: Expectation) -> Self {
        Self {
            dockerfile_name: dockerfile_name.to_string(),
            expectation,
            timeout_secs: None,
        }
    }

    pub fn with_timeout(mut self, secs: u64) -> Self {
        self.timeout_secs = Some(secs);
        self
    }

    pub async fn validate(&self) -> Result<TestCase, String> {
        let executor = DockerExecutor::new()?;

        // get workspace from project state
        let workspace = get_workspace();
        let workspace_str = workspace.to_string_lossy();

        // run the container
        let result = executor
            .run(&self.dockerfile_name, &workspace_str, self.timeout_secs)
            .await?;

        // interpret result based on expectation
        let test_result = match &self.expectation {
            Expectation::ExitCode(expected) => {
                if result.exit_code == *expected {
                    Ok(format!("exit code {} as expected", expected))
                } else {
                    let preview = truncate_output(&result.stderr, 500);
                    Err(format!(
                        "expected exit code {}, got {}\n{}",
                        expected, result.exit_code, preview
                    ))
                }
            }

            Expectation::FailIfStdoutContains(pattern) => {
                if result.stdout.contains(pattern) {
                    Err(format!("stdout contains '{}' (failure condition)", pattern))
                } else {
                    Ok("validation passed".to_string())
                }
            }

            Expectation::FailIfStderrContains(pattern) => {
                if result.stderr.contains(pattern) {
                    let preview = extract_context(&result.stderr, pattern, 200);
                    Err(format!("stderr contains '{}':\n{}", pattern, preview))
                } else {
                    Ok("validation passed".to_string())
                }
            }

            Expectation::PassIfStdoutContains(pattern) => {
                if result.stdout.contains(pattern) {
                    Ok(format!("stdout contains '{}' as expected", pattern))
                } else {
                    Err(format!("expected stdout to contain '{}'", pattern))
                }
            }

            Expectation::PassIfStderrContains(pattern) => {
                if result.stderr.contains(pattern) {
                    Ok(format!("stderr contains '{}' as expected", pattern))
                } else {
                    Err(format!("expected stderr to contain '{}'", pattern))
                }
            }
        };

        Ok(TestCase {
            name: format!("docker:{}", self.dockerfile_name),
            result: test_result,
        })
    }
}

/// get workspace path from lab state
fn get_workspace() -> PathBuf {
    let config = match Config::load() {
        Ok(c) => c,
        Err(_) => return PathBuf::from("."),
    };

    if !config.has_auth_token() {
        return PathBuf::from(".");
    }

    let state = match LabState::load(config.expose_token()) {
        Ok(s) => s,
        Err(_) => return PathBuf::from("."),
    };

    match state.get_active() {
        Some(lab) => PathBuf::from(&lab.workspace),
        None => PathBuf::from("."),
    }
}

/// truncate output for error messages
fn truncate_output(s: &str, max_len: usize) -> String {
    if s.len() <= max_len {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len])
    }
}

/// extract context around a pattern match
fn extract_context(s: &str, pattern: &str, context_chars: usize) -> String {
    if let Some(pos) = s.find(pattern) {
        let start = pos.saturating_sub(context_chars / 2);
        let end = (pos + pattern.len() + context_chars / 2).min(s.len());
        let excerpt = &s[start..end];

        if start > 0 || end < s.len() {
            format!("...{}...", excerpt)
        } else {
            excerpt.to_string()
        }
    } else {
        truncate_output(s, context_chars)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exit_code() {
        let exp = Expectation::parse("exit:0").unwrap();
        assert!(matches!(exp, Expectation::ExitCode(0)));

        let exp = Expectation::parse("exit:1").unwrap();
        assert!(matches!(exp, Expectation::ExitCode(1)));
    }

    #[test]
    fn test_parse_fail_if_stderr() {
        let exp = Expectation::parse("fail_if:stderr contains DATA RACE").unwrap();
        assert!(matches!(exp, Expectation::FailIfStderrContains(s) if s == "DATA RACE"));
    }

    #[test]
    fn test_parse_fail_if_stdout() {
        let exp = Expectation::parse("fail_if:stdout contains ERROR").unwrap();
        assert!(matches!(exp, Expectation::FailIfStdoutContains(s) if s == "ERROR"));
    }

    #[test]
    fn test_parse_pass_if_stdout() {
        let exp = Expectation::parse("pass_if:stdout contains SUCCESS").unwrap();
        assert!(matches!(exp, Expectation::PassIfStdoutContains(s) if s == "SUCCESS"));
    }

    #[test]
    fn test_parse_pass_if_stderr() {
        let exp = Expectation::parse("pass_if:stderr contains warning").unwrap();
        assert!(matches!(exp, Expectation::PassIfStderrContains(s) if s == "warning"));
    }

    #[test]
    fn test_parse_invalid() {
        assert!(Expectation::parse("invalid").is_err());
        assert!(Expectation::parse("exit:abc").is_err());
    }

    #[test]
    fn test_truncate_output() {
        let short = "hello";
        assert_eq!(truncate_output(short, 10), "hello");

        let long = "hello world this is a long string";
        let truncated = truncate_output(long, 10);
        assert!(truncated.len() <= 13); // 10 + "..."
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_docker_validator_new() {
        let v = DockerValidator::new("Go1.22", Expectation::ExitCode(0));
        assert_eq!(v.dockerfile_name, "Go1.22");
        assert!(v.timeout_secs.is_none());
    }

    #[test]
    fn test_docker_validator_with_timeout() {
        let v = DockerValidator::new("Go1.22", Expectation::ExitCode(0)).with_timeout(60);
        assert_eq!(v.timeout_secs, Some(60));
    }
}
