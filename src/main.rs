use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

use lux::{
    api::LighthouseAPIClient,
    auth::TokenAuthenticator,
    commands,
    config::Config,
    greet,
    message::Message,
    oops,
    VERSION,
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

    /// List available projects or get project details
    Projects {
        #[arg(short = 's', long)]
        slug: Option<String>,
    },

    /// Manage active project
    Project {
        #[command(subcommand)]
        action: ProjectAction,
    },

    /// List tasks for active project
    Tasks {
        #[arg(short = 'r', long)]
        refresh: bool,
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

    /// List hints for a task
    Hints {
        #[arg(short = 't', long)]
        task: String,
    },

    /// Manage hints (unlock)
    Hint {
        #[command(subcommand)]
        action: HintAction,
    },
}

#[derive(Subcommand)]
enum ProjectAction {
    /// Start working on a project
    Start {
        #[arg(short = 's', long)]
        slug: String,
    },
    /// Show active project status
    Status,
    /// Stop working on active project
    Stop,
}

#[derive(Subcommand)]
enum HintAction {
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

        Commands::Projects { slug } => {
            let config = Config::load()?;
            if !config.has_auth_token() {
                oops!("not authenticated. Run: `{}`", Commands::AUTH_USAGE);
                return Ok(());
            }

            let client = LighthouseAPIClient::from_config(&config);

            match slug {
                Some(slug) => match client.project_by_slug(&slug).await {
                    Ok(project) => {
                        Message::print_project_detail(&project);
                    }
                    Err(err) => {
                        oops!("failed to fetch project: {}", err);
                    }
                },
                None => match client.projects(None, None).await {
                    Ok(response) => {
                        Message::print_projects(&response);
                    }
                    Err(err) => {
                        oops!("failed to fetch projects: {}", err);
                    }
                },
            }
        }

        Commands::Project { action } => match action {
            ProjectAction::Start { slug } => {
                commands::project::start(&slug).await?;
            }
            ProjectAction::Status => {
                commands::project::status()?;
            }
            ProjectAction::Stop => {
                commands::project::stop()?;
            }
        },

        Commands::Tasks { refresh } => {
            commands::tasks::list(refresh).await?;
        }

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

        Commands::Hints { task } => {
            commands::hints::list(&task).await?;
        }

        Commands::Hint { action } => match action {
            HintAction::Unlock { task, hint } => {
                commands::hints::unlock(&task, &hint).await?;
            }
        },
    }

    Ok(())
}
