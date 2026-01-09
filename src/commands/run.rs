use color_eyre::eyre::Result;

use crate::api::{LighthouseAPIClient, SubmitAttemptRequest, Task, TaskOutcome};
use crate::config::Config;
use crate::message::Message;
use crate::state::ProjectState;
use crate::tasks::TestResults;
use crate::validators::create_validator;
use crate::{cheer, complain, oops, say};

/// handle `lux run --task <slug> [--project <slug>]`
pub async fn run(task_slug: &str, project_slug: Option<&str>, detailed: bool) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `lux auth --token <TOKEN>`");
        return Ok(());
    }

    let state = ProjectState::load(config.expose_token())?;
    let client = LighthouseAPIClient::from_config(&config);

    // determine project slug (from arg or active project)
    let project_slug = match project_slug {
        Some(s) => s.to_string(),
        None => match state.get_active() {
            Some(p) => p.slug.clone(),
            None => {
                oops!("no project specified and no active project");
                say!("use `--project <SLUG>` or run `lux project start --slug <SLUG>` first");
                return Ok(());
            }
        },
    };

    // fetch project with tasks
    let project_data = match client.project_by_slug(&project_slug).await {
        Ok(p) => p,
        Err(err) => {
            oops!("failed to fetch project '{}': {}", project_slug, err);
            return Ok(());
        }
    };

    // find the task by slug
    let tasks = match &project_data.tasks {
        Some(t) => t,
        None => {
            oops!("project '{}' has no tasks", project_slug);
            return Ok(());
        }
    };

    let task_data = match tasks.iter().find(|t| t.slug == task_slug) {
        Some(t) => t,
        None => {
            oops!("task '{}' not found in project '{}'", task_slug, project_slug);
            say!("available tasks:");
            for t in tasks {
                say!("  - {}", t.slug);
            }
            return Ok(());
        }
    };

    run_task_validators(&client, &project_data.slug, task_data, detailed).await
}

/// run validators for a single task and submit results
pub async fn run_task_validators(
    client: &LighthouseAPIClient,
    project_slug: &str,
    task: &Task,
    detailed: bool,
) -> Result<()> {
    // check if task already completed
    let already_passed = task.status == "challenge_completed";
    if already_passed {
        complain!("you've already passed this task");
        say!("running validators anyway for verification...");
    }

    // print task info
    Message::print_task_header(task, detailed);

    // run validators
    if task.validators.is_empty() {
        say!("no validators defined for this task");
        return Ok(());
    }

    Message::print_validators_start(task.validators.len());

    let mut results = TestResults::new();

    for (i, validator_str) in task.validators.iter().enumerate() {
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
        project_slug: project_slug.to_string(),
        task_id: task.id,
        task_outcome: outcome,
        points_achieved: None,
        task_outcome_context: Some(context),
    };

    match client.submit_attempt(&attempt_request).await {
        Ok(response) => {
            log::debug!("attempt recorded: {:?}", response);
            if response.data.is_reattempt {
                say!("re-attempt recorded (no additional points)");
            } else if response.data.task_outcome == "passed" {
                cheer!("task completed! +{} points", response.data.points_achieved);
            } else {
                say!("attempt recorded");
            }
        }
        Err(err) => {
            log::error!("failed to submit attempt: {}", err);
            oops!("failed to submit results: {}", err);
        }
    }

    Ok(())
}
