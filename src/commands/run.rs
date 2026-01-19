use color_eyre::eyre::Result;

use crate::api::{LighthouseAPIClient, SubmitAttemptRequest, Task, TaskOutcome, TaskStatus};
use crate::config::Config;
use crate::shell;
use crate::state::ProjectState;
use crate::tasks::{TestCase, TestResults};
use crate::ui::RunUI;
use crate::validators::create_validator;
use crate::{complain, oops, say};

/// handle `luxctlrun --task <slug|number> [--project <slug>]`
/// task can be specified by slug or by number (1, 01, 2, 02, etc.)
pub async fn run(task_id: &str, project_slug: Option<&str>, detailed: bool) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `luxctl auth --token $token`");
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
                say!("use `--project <SLUG>` or run `luxctlproject start --slug <SLUG>` first");
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
            oops!("task '{}' not found in project '{}'", task_id, project_slug);
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
    _detailed: bool,
    state_ctx: Option<(&mut ProjectState, &str)>,
) -> Result<()> {
    let ui = RunUI::new(&task.slug, task.validators.len());

    // check if task already completed
    let already_passed = task.status.is_completed();
    if already_passed {
        complain!("you've already passed this task");
        say!("running validators anyway for verification...");
    }

    ui.header();
    ui.blank_line();

    // run prologue commands
    if !task.prologue.is_empty() {
        ui.step(&format!("Running {} prologue commands...", task.prologue.len()));
        if let Err((cmd, result)) = shell::run_commands(&task.prologue).await {
            oops!("prologue command failed: {}", cmd);
            if !result.stderr.is_empty() {
                say!("stderr: {}", result.stderr.trim());
            }
            // run epilogue for cleanup even if prologue fails
            run_epilogue(&ui, &task.epilogue).await;
            return Ok(());
        }
        ui.blank_line();
    }

    // run validators
    if task.validators.is_empty() {
        ui.step("no validators defined for this task");
        run_epilogue(&ui, &task.epilogue).await;
        return Ok(());
    }

    ui.step(&format!("Running {} validators...", task.validators.len()));
    ui.blank_line();

    let mut results = TestResults::new();

    for validator_str in task.validators.iter() {
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
                if test_case.passed() {
                    ui.test_pass(&test_case.name);
                } else {
                    let detail = if test_case.message() != test_case.name {
                        Some(test_case.message())
                    } else {
                        None
                    };
                    ui.test_fail(&test_case.name, detail);
                }
                results.add(test_case);
            }
            Err(err) => {
                ui.test_fail(&err, None);
                let failed_case = TestCase {
                    name: err.clone(),
                    result: Err(err),
                };
                results.add(failed_case);
            }
        }
    }

    ui.blank_line();
    if results.all_passed() {
        ui.summary_pass(results.total());
    } else {
        ui.summary_fail(results.passed(), results.total());

        // show hints from task if available
        if !task.hints.is_empty() {
            for hint in &task.hints {
                ui.hint(&hint.text);
            }
        }
    }

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
                ui.points_earned(response.data.points_achieved);
            }

            // update cached task status if state context provided
            if let Some((state, token)) = state_ctx {
                let new_status = if response.data.task_outcome == "passed" {
                    TaskStatus::ChallengeCompleted
                } else {
                    TaskStatus::Challenged
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

    // run epilogue commands (cleanup)
    run_epilogue(&ui, &task.epilogue).await;

    Ok(())
}

/// run epilogue commands with best-effort (continues even on failure)
async fn run_epilogue(ui: &RunUI, commands: &[String]) {
    if commands.is_empty() {
        return;
    }

    ui.blank_line();
    ui.step(&format!("Running {} epilogue commands...", commands.len()));

    let failures = shell::run_commands_best_effort(commands).await;
    for (cmd, result) in failures {
        log::warn!("epilogue command failed: {} (exit {})", cmd, result.exit_code);
        if !result.stderr.is_empty() {
            log::debug!("stderr: {}", result.stderr.trim());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::TaskStatus;

    fn make_task_with_hooks(
        prologue: Vec<String>,
        epilogue: Vec<String>,
        validators: Vec<String>,
    ) -> Task {
        Task {
            id: 1,
            slug: "test-task".to_string(),
            title: "Test Task".to_string(),
            description: "A test task".to_string(),
            sort_order: 1,
            scores: "10:20:50".to_string(),
            status: TaskStatus::ChallengeAwaits,
            is_locked: false,
            abandoned_deduction: 5,
            points_earned: 0,
            hints: vec![],
            validators,
            prologue,
            epilogue,
        }
    }

    #[test]
    fn test_task_with_empty_hooks() {
        let task = make_task_with_hooks(vec![], vec![], vec![]);
        assert!(task.prologue.is_empty());
        assert!(task.epilogue.is_empty());
    }

    #[test]
    fn test_task_with_prologue_and_epilogue() {
        let task = make_task_with_hooks(
            vec!["docker compose up -d".to_string()],
            vec!["docker compose down".to_string()],
            vec!["tcp_listening:int(8080)".to_string()],
        );

        assert_eq!(task.prologue.len(), 1);
        assert_eq!(task.epilogue.len(), 1);
        assert_eq!(task.prologue[0], "docker compose up -d");
        assert_eq!(task.epilogue[0], "docker compose down");
    }

    #[tokio::test]
    async fn test_prologue_stops_on_failure() {
        let commands = vec![
            "echo starting".to_string(),
            "exit 1".to_string(),
            "echo should not run".to_string(),
        ];

        let result = shell::run_commands(&commands).await;
        assert!(result.is_err());

        let (failed_cmd, _) = result.unwrap_err();
        assert_eq!(failed_cmd, "exit 1");
    }

    #[tokio::test]
    async fn test_epilogue_continues_on_failure() {
        let commands = vec![
            "exit 1".to_string(),
            "exit 2".to_string(),
            "echo still runs".to_string(),
        ];

        // best_effort continues even when commands fail
        let failures = shell::run_commands_best_effort(&commands).await;

        // should have 2 failures (exit 1 and exit 2)
        assert_eq!(failures.len(), 2);
    }

    #[tokio::test]
    async fn test_prologue_success_allows_continuation() {
        let commands = vec!["echo one".to_string(), "echo two".to_string()];

        let result = shell::run_commands(&commands).await;
        assert!(result.is_ok());
    }
}
