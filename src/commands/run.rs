use color_eyre::eyre::Result;

use crate::api::{LighthouseAPIClient, SubmitAttemptRequest, Task, TaskOutcome};
use crate::config::Config;
use crate::message::Message;
use crate::state::ProjectState;
use crate::tasks::{TestCase, TestResults};
use crate::validators::create_validator;
use crate::{cheer, complain, oops, say};

/// handle `lux run --task <slug|number> [--project <slug>]`
/// task can be specified by slug or by number (1, 01, 2, 02, etc.)
pub async fn run(task_id: &str, project_slug: Option<&str>, detailed: bool) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `lux auth --token <TOKEN>`");
        return Ok(());
    }

    let token = config.expose_token().to_string();
    let mut state = ProjectState::load(&token)?;
    let client = LighthouseAPIClient::from_config(&config);

    // determine project slug (from arg or active project)
    let project_slug = match project_slug {
        Some(s) => s.to_string(),
        None => {
            if let Some(p) = state.get_active() {
                p.slug.clone()
            } else {
                oops!("no project specified and no active project");
                say!("use `--project <SLUG>` or run `lux project start --slug <SLUG>` first");
                return Ok(());
            }
        }
    };

    // fetch project with tasks
    let project_data = match client.project_by_slug(&project_slug).await {
        Ok(p) => p,
        Err(err) => {
            oops!("failed to fetch project '{}': {}", project_slug, err);
            return Ok(());
        }
    };

    // get tasks list
    let tasks = if let Some(t) = &project_data.tasks {
        t
    } else {
        oops!("project '{}' has no tasks", project_slug);
        return Ok(());
    };

    // find task by number or slug
    let task_data = if let Ok(task_num) = task_id.parse::<usize>() {
        // task specified by number (1-based index)
        if task_num == 0 || task_num > tasks.len() {
            oops!(
                "task #{} not found. valid range: 1-{}",
                task_num,
                tasks.len()
            );
            return Ok(());
        }
        &tasks[task_num - 1]
    } else {
        // task specified by slug
        if let Some(t) = tasks.iter().find(|t| t.slug == task_id) {
            t
        } else {
            oops!(
                "task '{}' not found in project '{}'",
                task_id,
                project_slug
            );
            say!("use task number (1, 2, 3...) or slug:");
            for (i, t) in tasks.iter().enumerate() {
                say!("  {:02}. {}", i + 1, t.slug);
            }
            return Ok(());
        }
    };

    run_task_validators(
        &client,
        &project_data.slug,
        task_data,
        detailed,
        Some((&mut state, &token)),
    )
    .await
}

/// run validators for a single task and submit results
/// optionally updates cached state when state_ctx is provided
pub async fn run_task_validators(
    client: &LighthouseAPIClient,
    project_slug: &str,
    task: &Task,
    detailed: bool,
    state_ctx: Option<(&mut ProjectState, &str)>,
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
                // record as failed test case with error as the name
                let failed_case = TestCase {
                    name: err.clone(),
                    result: Err(err),
                };
                Message::print_test_case(&failed_case, i);
                results.add(failed_case);
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

    // truncate context if too long (API limit is 5000 chars)
    let context = if context.len() > 4900 {
        format!("{}...[truncated]", &context[..4900])
    } else {
        context
    };

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
                log::debug!("re-attempt recorded (no additional points)");
            } else if response.data.task_outcome == "passed" {
                cheer!("task completed! +{} points", response.data.points_achieved);
            }

            // update cached task status if state context provided
            if let Some((state, token)) = state_ctx {
                let new_status = if response.data.task_outcome == "passed" {
                    "challenge_completed"
                } else {
                    "challenged"
                };
                state.update_task_status(task.id, new_status);
                if let Err(e) = state.save(token) {
                    log::warn!("failed to save state: {}", e);
                }
            }
        }
        Err(err) => {
            log::error!("failed to submit attempt: {}", err);
            oops!("failed to submit results: {}", err);
        }
    }

    Ok(())
}
