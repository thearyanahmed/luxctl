//! `lux doctor` - diagnose environment and check tool availability

use color_eyre::eyre::Result;
use colored::Colorize;
use std::process::Command;

use crate::api::LighthouseAPIClient;
use crate::config::Config;
use crate::state::ProjectState;
use crate::{say, VERSION};

/// check result for a single diagnostic
struct CheckResult {
    name: String,
    status: CheckStatus,
    detail: Option<String>,
}

enum CheckStatus {
    Ok,
    Warning,
    Error,
    NotInstalled,
}

impl CheckResult {
    fn ok(name: &str, detail: Option<String>) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Ok,
            detail,
        }
    }

    fn warning(name: &str, detail: Option<String>) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Warning,
            detail,
        }
    }

    fn error(name: &str, detail: Option<String>) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::Error,
            detail,
        }
    }

    fn not_installed(name: &str) -> Self {
        Self {
            name: name.to_string(),
            status: CheckStatus::NotInstalled,
            detail: None,
        }
    }

    fn print(&self) {
        let (icon, colored_name) = match self.status {
            CheckStatus::Ok => ("[X]".green().to_string(), self.name.green().to_string()),
            CheckStatus::Warning => ("[!]".yellow().to_string(), self.name.yellow().to_string()),
            CheckStatus::Error => ("[X]".red().to_string(), self.name.red().to_string()),
            CheckStatus::NotInstalled => ("[O]".dimmed().to_string(), self.name.dimmed().to_string()),
        };

        match &self.detail {
            Some(d) => println!("  {} {}  {}", icon, colored_name, d.dimmed()),
            None => println!("  {} {}", icon, colored_name),
        }
    }
}

/// run all diagnostic checks
pub async fn run() -> Result<()> {
    say!("lux doctor v{}\n", VERSION);

    // system info
    print_section("System");
    check_system_info();

    // authentication
    print_section("Authentication");
    let config = check_auth();

    // network connectivity
    print_section("Network");
    check_network(&config).await;

    // development tools
    print_section("Development Tools");
    check_dev_tools();

    // active project
    print_section("Project State");
    check_project_state(&config);

    println!();
    say!("run `lux doctor` after installing missing tools to verify");

    Ok(())
}

fn print_section(name: &str) {
    println!("\n{}:", name.bold());
}

fn check_system_info() {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;

    CheckResult::ok("os", Some(os.to_string())).print();
    CheckResult::ok("arch", Some(arch.to_string())).print();

    // check home directory exists (needed for config storage)
    match dirs::home_dir() {
        Some(home) => {
            let lux_dir = home.join(".lux");
            if lux_dir.exists() {
                CheckResult::ok("config dir", Some(lux_dir.to_string_lossy().to_string())).print();
            } else {
                CheckResult::warning(
                    "config dir",
                    Some(format!("{} (will be created)", lux_dir.to_string_lossy())),
                )
                .print();
            }
        }
        None => {
            CheckResult::error(
                "home dir",
                Some("could not determine home directory".into()),
            )
            .print();
        }
    }
}

fn check_auth() -> Option<Config> {
    match Config::load() {
        Ok(config) if config.has_auth_token() => {
            CheckResult::ok("logged in", Some("token configured".into())).print();
            Some(config)
        }
        Ok(_) => {
            CheckResult::warning(
                "not logged in",
                Some("run `lux auth --token <TOKEN>`".into()),
            )
            .print();
            None
        }
        Err(e) => {
            CheckResult::error("config", Some(format!("failed to load: {}", e))).print();
            None
        }
    }
}

async fn check_network(config: &Option<Config>) {
    // check if we can reach the API
    let Some(config) = config else {
        CheckResult::warning("api", Some("skipped (not authenticated)".into())).print();
        return;
    };

    let client = LighthouseAPIClient::from_config(config);
    match client.me().await {
        Ok(user) => {
            CheckResult::ok("api", Some(format!("connected as {}", user.email))).print();
        }
        Err(e) => {
            let msg = format!("{}", e);
            if msg.contains("timeout") || msg.contains("connect") {
                CheckResult::error(
                    "api",
                    Some("could not connect to projectlighthouse.io".into()),
                )
                .print();
            } else {
                CheckResult::error("api", Some(msg)).print();
            }
        }
    }
}

fn check_dev_tools() {
    // these are the tools validators may need
    let tools = vec![
        ToolCheck::new("git", &["--version"], true),
        ToolCheck::new("go", &["version"], false),
        ToolCheck::new("cargo", &["--version"], false),
        ToolCheck::new("rustc", &["--version"], false),
        ToolCheck::new("gcc", &["--version"], false),
        ToolCheck::new("make", &["--version"], false),
        ToolCheck::new("docker", &["--version"], false),
    ];

    for tool in tools {
        tool.check().print();
    }

    println!();
    say!(
        "  {}",
        "note: only install tools needed for your chosen runtime".dimmed()
    );
}

struct ToolCheck {
    name: &'static str,
    args: &'static [&'static str],
    required: bool,
}

impl ToolCheck {
    fn new(name: &'static str, args: &'static [&'static str], required: bool) -> Self {
        Self {
            name,
            args,
            required,
        }
    }

    fn check(&self) -> CheckResult {
        match Command::new(self.name).args(self.args).output() {
            Ok(output) if output.status.success() => {
                let version = extract_version(&output.stdout);
                CheckResult::ok(self.name, version)
            }
            Ok(_) => {
                // command exists but returned error
                if self.required {
                    CheckResult::error(self.name, Some("installed but returned error".into()))
                } else {
                    CheckResult::warning(self.name, Some("installed but returned error".into()))
                }
            }
            Err(_) => {
                if self.required {
                    CheckResult::error(self.name, Some("required but not found".into()))
                } else {
                    CheckResult::not_installed(self.name)
                }
            }
        }
    }
}

/// extract version string from command output
fn extract_version(output: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(output);
    let first_line = text.lines().next()?;

    // try to find version-like pattern (e.g., "1.2.3", "v1.2.3", "go1.22.0")
    // common patterns:
    // - "git version 2.39.0"
    // - "go version go1.22.0 darwin/arm64"
    // - "cargo 1.75.0"
    // - "rustc 1.75.0"
    // - "Docker version 24.0.7, build abcd123"
    // - "gcc (Homebrew GCC 13.2.0) 13.2.0"
    // - "GNU Make 3.81"

    // simple approach: find words containing digits and dots
    for word in first_line.split_whitespace() {
        let cleaned = word
            .trim_start_matches('v')
            .trim_start_matches("go")
            .trim_end_matches(',');
        if cleaned.contains('.') && cleaned.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return Some(cleaned.to_string());
        }
    }

    // fallback: just return the first line trimmed
    Some(first_line.trim().to_string())
}

fn check_project_state(config: &Option<Config>) {
    let Some(config) = config else {
        CheckResult::warning("project", Some("skipped (not authenticated)".into())).print();
        return;
    };

    let state = match ProjectState::load(config.expose_token()) {
        Ok(s) => s,
        Err(e) => {
            CheckResult::error("state", Some(format!("failed to load: {}", e))).print();
            return;
        }
    };

    if let Some(project) = state.get_active() {
        CheckResult::ok("active project", Some(project.name.clone())).print();

        // check workspace exists
        let workspace_path = std::path::Path::new(&project.workspace);
        if workspace_path.exists() {
            CheckResult::ok("workspace", Some(project.workspace.clone())).print();
        } else {
            CheckResult::error(
                "workspace",
                Some(format!("{} (not found)", project.workspace)),
            )
            .print();
        }

        // show runtime if set
        if let Some(rt) = &project.runtime {
            CheckResult::ok("runtime", Some(rt.clone())).print();
        } else {
            CheckResult::warning("runtime", Some("not set".into())).print();
        }

        // show progress
        let progress = format!(
            "{}/{} tasks completed",
            project.completed_count(),
            project.tasks.len()
        );
        CheckResult::ok("progress", Some(progress)).print();
    } else {
        CheckResult::ok("project", Some("none active".into())).print();
        say!(
            "  {}",
            "run `lux project start --slug <SLUG>` to begin".dimmed()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_version_git() {
        let output = b"git version 2.39.0";
        assert_eq!(extract_version(output), Some("2.39.0".to_string()));
    }

    #[test]
    fn test_extract_version_go() {
        let output = b"go version go1.22.0 darwin/arm64";
        assert_eq!(extract_version(output), Some("1.22.0".to_string()));
    }

    #[test]
    fn test_extract_version_cargo() {
        let output = b"cargo 1.75.0 (abc123 2024-01-01)";
        assert_eq!(extract_version(output), Some("1.75.0".to_string()));
    }

    #[test]
    fn test_extract_version_docker() {
        let output = b"Docker version 24.0.7, build abcd123";
        assert_eq!(extract_version(output), Some("24.0.7".to_string()));
    }

    #[test]
    fn test_extract_version_make() {
        let output = b"GNU Make 3.81";
        assert_eq!(extract_version(output), Some("3.81".to_string()));
    }

    #[test]
    fn test_extract_version_fallback() {
        let output = b"some unknown format";
        assert_eq!(
            extract_version(output),
            Some("some unknown format".to_string())
        );
    }
}
