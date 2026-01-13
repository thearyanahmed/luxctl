use color_eyre::eyre::Result;

use crate::api::LighthouseAPIClient;
use crate::commands::run::run_task_validators;
use crate::config::Config;
use crate::state::ProjectState;
use crate::{oops, say};

/// handle `lux validate [--all] [--detailed]`
pub async fn validate_all(include_passed: bool, detailed: bool) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `lux auth --token <TOKEN>`");
        return Ok(());
    }

    let token = config.expose_token().to_string();
    let mut state = ProjectState::load(&token)?;

    let active = match state.get_active() {
        Some(p) => p.clone(),
        None => {
            oops!("no active project");
            say!("run `lux project start --slug <SLUG>` first");
            return Ok(());
        }
    };

    let client = LighthouseAPIClient::from_config(&config);

    // fetch fresh project data
    let project = match client.project_by_slug(&active.slug).await {
        Ok(p) => p,
        Err(err) => {
            oops!("failed to fetch project: {}", err);
            return Ok(());
        }
    };

    let tasks = match &project.tasks {
        Some(t) => t,
        None => {
            oops!("project has no tasks");
            return Ok(());
        }
    };

    // update cache with fresh data
    state.refresh_tasks(tasks);
    state.save(&token)?;

    // filter tasks
    let mut to_run = Vec::new();
    let mut skipped_completed = 0;
    let mut skipped_locked = 0;

    for task in tasks {
        let is_completed = task.status == "challenge_completed";
        // TODO: add proper access check when API provides it
        let is_locked = false;

        if is_locked {
            skipped_locked += 1;
            continue;
        }

        if is_completed && !include_passed {
            skipped_completed += 1;
            continue;
        }

        to_run.push(task);
    }

    say!("validating tasks for: {}", project.name);

    if skipped_completed > 0 {
        say!(
            "skipping {} completed task(s). Use --all to include them.",
            skipped_completed
        );
    }

    if to_run.is_empty() {
        say!("no tasks to validate");
        return Ok(());
    }

    // run each task
    for (i, task) in to_run.iter().enumerate() {
        println!();
        println!(
            "━━━ Task {}/{}: {} ━━━━━━━━━━━━━━━━━━━━",
            i + 1,
            to_run.len(),
            task.slug
        );

        // run validators and submit results (pass state for auto-refresh)
        run_task_validators(
            &client,
            &project.slug,
            task,
            detailed,
            Some((&mut state, &token)),
        )
        .await?;
    }

    // print summary
    println!();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    say!("summary");
    say!("  ran: {} task(s)", to_run.len());
    if skipped_completed > 0 {
        say!("  skipped: {} (completed)", skipped_completed);
    }
    if skipped_locked > 0 {
        say!("  skipped: {} (locked)", skipped_locked);
    }

    Ok(())
}
