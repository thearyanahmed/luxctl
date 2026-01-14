use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

use lux::{
    api::LighthouseAPIClient, auth::TokenAuthenticator, commands, config::Config, greet,
    message::Message, oops, VERSION,
};

#[derive(Parser)]
#[command(name = "lux")]
#[command(version = VERSION)]
struct Cli {
    #[command(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with your API token
    Auth {
        #[arg(short = 't', long)]
        token: String,
    },

    /// Show current authenticated user
    Whoami,

    /// Manage projects
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },

    /// Manage tasks
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },

    /// Run validators for a specific task
    Run {
        #[arg(short = 'p', long)]
        project: Option<String>,

        #[arg(short = 't', long)]
        task: String,

        #[arg(short = 'd', long)]
        detailed: bool,
    },

    /// Validate all tasks in active project
    Validate {
        #[arg(short = 'd', long)]
        detailed: bool,

        #[arg(short = 'a', long)]
        all: bool,
    },

    /// Manage hints
    Hint {
        #[command(subcommand)]
        action: HintAction,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// List available projects
    List,
    /// Show project details
    Show {
        #[arg(short = 's', long)]
        slug: String,
    },
    /// Start working on a project
    Start {
        #[arg(short = 's', long)]
        slug: String,

        /// Workspace directory (defaults to current directory)
        #[arg(short = 'w', long, default_value = ".")]
        workspace: String,

        /// Runtime environment (go, rust, c, python)
        #[arg(short = 'r', long)]
        runtime: Option<String>,
    },
    /// Show active project status
    Status,
    /// Stop working on active project
    Stop,
    /// Update project settings
    Set {
        /// Runtime environment (go, rust, c, python)
        #[arg(short = 'r', long)]
        runtime: String,
    },
}

#[derive(Subcommand)]
enum TaskAction {
    /// List tasks for active project
    List {
        #[arg(short = 'r', long)]
        refresh: bool,
    },
    /// Show task details
    Show {
        /// Task number or slug
        #[arg(short = 't', long)]
        task: String,

        /// Show full description
        #[arg(short = 'd', long)]
        detailed: bool,
    },
}

#[derive(Subcommand)]
enum HintAction {
    /// List hints for a task
    List {
        #[arg(short = 't', long)]
        task: String,
    },
    /// Unlock a hint (deducts points)
    Unlock {
        #[arg(short = 't', long)]
        task: String,

        #[arg(short = 'i', long)]
        hint: String,
    },
}

impl Commands {
    pub const AUTH_USAGE: &'static str = "lux auth --token <TOKEN>";
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    env_logger::init();

    let cli = Cli::parse();

    match cli.commands {
        Commands::Auth { token } => {
            let authenticator = TokenAuthenticator::new(&token);

            match authenticator.authenticate().await {
                Ok(user) => {
                    greet!(user.name());
                }
                Err(err) => {
                    log::error!("{}", err);
                    oops!("{}", err);
                }
            }
        }

        Commands::Whoami => {
            let config = match Config::load() {
                Ok(c) if c.has_auth_token() => c,
                _ => {
                    println!("nobody");
                    println!("login with: {}", Commands::AUTH_USAGE);
                    return Ok(());
                }
            };

            let client = LighthouseAPIClient::from_config(&config);
            match client.me().await {
                Ok(user) => {
                    println!("{}", user.name);
                    println!("{}", user.email);
                    if let Some(stats) = user.stats {
                        println!();
                        println!("projects: {}", stats.projects_attempted);
                        println!("tasks completed: {}", stats.tasks_completed);
                        println!("total xp: {}", stats.total_xp);
                    }
                }
                Err(err) => {
                    oops!("failed to fetch user: {}", err);
                }
            }
        }

        Commands::Project { action } => match action {
            ProjectAction::List => {
                let config = Config::load()?;
                if !config.has_auth_token() {
                    oops!("not authenticated. Run: `{}`", Commands::AUTH_USAGE);
                    return Ok(());
                }

                let client = LighthouseAPIClient::from_config(&config);
                match client.projects(None, None).await {
                    Ok(response) => {
                        Message::print_projects(&response);
                    }
                    Err(err) => {
                        oops!("failed to fetch projects: {}", err);
                    }
                }
            }
            ProjectAction::Show { slug } => {
                let config = Config::load()?;
                if !config.has_auth_token() {
                    oops!("not authenticated. Run: `{}`", Commands::AUTH_USAGE);
                    return Ok(());
                }

                let client = LighthouseAPIClient::from_config(&config);
                match client.project_by_slug(&slug).await {
                    Ok(project) => {
                        Message::print_project_detail(&project);
                    }
                    Err(err) => {
                        oops!("failed to fetch project: {}", err);
                    }
                }
            }
            ProjectAction::Start {
                slug,
                workspace,
                runtime,
            } => {
                commands::project::start(&slug, &workspace, runtime.as_deref()).await?;
            }
            ProjectAction::Status => {
                commands::project::status()?;
            }
            ProjectAction::Stop => {
                commands::project::stop()?;
            }
            ProjectAction::Set { runtime } => {
                commands::project::set_runtime(&runtime)?;
            }
        },

        Commands::Task { action } => match action {
            TaskAction::List { refresh } => {
                commands::tasks::list(refresh).await?;
            }
            TaskAction::Show { task, detailed } => {
                commands::task::show(&task, detailed).await?;
            }
        },

        Commands::Run {
            project,
            task,
            detailed,
        } => {
            commands::run::run(&task, project.as_deref(), detailed).await?;
        }

        Commands::Validate { detailed, all } => {
            commands::validate::validate_all(all, detailed).await?;
        }

        Commands::Hint { action } => match action {
            HintAction::List { task } => {
                commands::hints::list(&task).await?;
            }
            HintAction::Unlock { task, hint } => {
                commands::hints::unlock(&task, &hint).await?;
            }
        },
    }

    Ok(())
}
