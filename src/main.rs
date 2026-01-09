use clap::{Parser, Subcommand};
use color_eyre::eyre::Result;

use lux::{
    api::{LighthouseAPIClient, SubmitAttemptRequest, TaskOutcome},
    auth::TokenAuthenticator,
    cheer, complain,
    config::Config,
    greet,
    message::Message,
    oops, say,
    tasks::TestResults,
    validators::create_validator,
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
    Auth {
        #[arg(short = 't', long)]
        token: String,
    },

    Run {
        #[arg(short = 'p', long)]
        project: String,

        #[arg(short = 't', long)]
        task: String,

        #[arg(short = 'd', long)]
        detailed: bool,
    },

    Projects {
        #[arg(short = 's', long)]
        slug: Option<String>,
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
        Commands::Run { project, task, detailed } => {
            let config = Config::load()?;
            if !config.has_auth_token() {
                oops!("not authenticated. Run: `{}`", Commands::AUTH_USAGE);
                return Ok(());
            }

            let client = LighthouseAPIClient::from_config(&config);

            // fetch project with tasks
            let project_data = match client.project_by_slug(&project).await {
                Ok(p) => p,
                Err(err) => {
                    oops!("failed to fetch project '{}': {}", project, err);
                    return Ok(());
                }
            };

            // find the task by slug
            let tasks = match &project_data.tasks {
                Some(t) => t,
                None => {
                    oops!("project '{}' has no tasks", project);
                    return Ok(());
                }
            };

            let task_data = match tasks.iter().find(|t| t.slug == task) {
                Some(t) => t,
                None => {
                    oops!("task '{}' not found in project '{}'", task, project);
                    say!("available tasks:");
                    for t in tasks {
                        say!("  - {}", t.slug);
                    }
                    return Ok(());
                }
            };

            // check if task already completed
            let already_passed = task_data.status == "challenge_completed";
            if already_passed {
                complain!("you've already passed this task");
                say!("running validators anyway for verification...");
            }

            // print task info
            Message::print_task_header(task_data, detailed);

            // run validators
            if task_data.validators.is_empty() {
                say!("no validators defined for this task");
                return Ok(());
            }

            Message::print_validators_start(task_data.validators.len());

            let mut results = TestResults::new();

            for (i, validator_str) in task_data.validators.iter().enumerate() {
                log::debug!("parsing validator: {}", validator_str);

                let validator = match create_validator(validator_str) {
                    Ok(v) => v,
                    Err(err) => {
                        oops!("invalid validator '{}': {}", validator_str, err);
                        continue;
                    }
                };

                match validator.validate().await {
                    Ok(test_case) => {
                        Message::print_test_case(&test_case, i);
                        results.add(test_case);
                    }
                    Err(err) => {
                        // connection errors get special treatment
                        if err.contains("connection failed") || err.contains("connection timeout") {
                            Message::print_connection_error(4221);
                            return Ok(());
                        }
                        oops!("validator error: {}", err);
                    }
                }
            }

            Message::print_test_results(&results);

            // report results back to API
            let outcome = if results.all_passed() {
                TaskOutcome::Passed
            } else {
                TaskOutcome::Failed
            };

            // build context string from test results
            let context = results
                .tests
                .iter()
                .enumerate()
                .map(|(i, t)| {
                    let status = if t.passed() { "PASS" } else { "FAIL" };
                    format!("#{} [{}] {}: {}", i + 1, status, t.name, t.message())
                })
                .collect::<Vec<_>>()
                .join("\n");

            let attempt_request = SubmitAttemptRequest {
                project_slug: project.clone(),
                task_id: task_data.id,
                task_outcome: outcome,
                points_achieved: None, // TODO: calculate based on scoring rules
                task_outcome_context: Some(context),
            };

            match client.submit_attempt(&attempt_request).await {
                Ok(response) => {
                    log::debug!("attempt recorded: {:?}", response);
                    if response.data.is_reattempt {
                        say!("re-attempt recorded (no additional points)");
                    } else if response.data.task_outcome == "passed" {
                        cheer!(
                            "task completed! +{} points",
                            response.data.points_achieved
                        );
                    } else {
                        say!("attempt recorded");
                    }
                }
                Err(err) => {
                    log::error!("failed to submit attempt: {}", err);
                    oops!("failed to submit results: {}", err);
                }
            }
        }
    }

    Ok(())
}
