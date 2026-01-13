use color_eyre::eyre::Result;

use crate::api::LighthouseAPIClient;
use crate::config::Config;
use crate::state::ProjectState;
use crate::{cheer, oops, say};

/// handle `lux project start --slug <slug>`
pub async fn start(slug: &str) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `lux auth --token <TOKEN>`");
        return Ok(());
    }

    let client = LighthouseAPIClient::from_config(&config);

    // fetch project to verify it exists and get task data
    let project = match client.project_by_slug(slug).await {
        Ok(p) => p,
        Err(err) => {
            oops!("project '{}' not found: {}", slug, err);
            say!("run `lux projects` to see available projects");
            return Ok(());
        }
    };

    let tasks = project.tasks.as_deref().unwrap_or(&[]);

    // save to state
    let mut state = ProjectState::load(config.expose_token())?;
    state.set_active(&project.slug, &project.name, tasks);
    state.save(config.expose_token())?;

    cheer!("now working on: {}", project.name);
    say!("run `lux tasks` to see available tasks");

    Ok(())
}

/// handle `lux project status`
pub fn status() -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `lux auth --token <TOKEN>`");
        return Ok(());
    }

    let state = ProjectState::load(config.expose_token())?;

    if let Some(project) = state.get_active() {
        say!("active project: {}", project.name);
        say!("       slug: {}", project.slug);
        say!(
            "   progress: {}/{} tasks completed",
            project.completed_count(),
            project.tasks.len()
        );
        say!("run `lux tasks` for task list");
    } else {
        say!("no active project");
        say!("run `lux project start --slug <SLUG>` to start one");
    }

    Ok(())
}

/// handle `lux project stop`
pub fn stop() -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `lux auth --token <TOKEN>`");
        return Ok(());
    }

    let mut state = ProjectState::load(config.expose_token())?;

    if state.get_active().is_some() {
        let name = state.get_active().map(|p| p.name.clone()).unwrap_or_default();
        state.clear_active();
        state.save(config.expose_token())?;
        say!("stopped working on: {}", name);
    } else {
        say!("no active project to stop");
    }

    Ok(())
}
