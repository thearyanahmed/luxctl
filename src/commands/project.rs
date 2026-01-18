use color_eyre::eyre::Result;

use crate::api::LighthouseAPIClient;
use crate::config::Config;
use crate::state::ProjectState;
use crate::{cheer, oops, say};

/// handle `luxctlproject start --slug <slug> --workspace <path> [--runtime <runtime>]`
pub async fn start(slug: &str, workspace: &str, runtime: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `luxctl auth --token $token`");
        return Ok(());
    }

    let client = LighthouseAPIClient::from_config(&config);

    // fetch project to verify it exists and get task data
    let project = match client.project_by_slug(slug).await {
        Ok(p) => p,
        Err(err) => {
            oops!("project '{}' not found: {}", slug, err);
            say!("run `luxctlprojects` to see available projects");
            return Ok(());
        }
    };

    // resolve workspace to absolute path
    let workspace_path = std::path::Path::new(workspace);
    let absolute_workspace = if workspace_path.is_absolute() {
        workspace_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| color_eyre::eyre::eyre!("cannot get cwd: {}", e))?
            .join(workspace_path)
    };

    let workspace_str = absolute_workspace.to_string_lossy().to_string();

    let tasks = project.tasks.as_deref().unwrap_or(&[]);

    // save to state
    let mut state = ProjectState::load(config.expose_token())?;
    state.set_active(&project.slug, &project.name, tasks, &workspace_str, runtime);
    state.save(config.expose_token())?;

    cheer!("now working on: {}", project.name);
    say!("  workspace: {}", workspace_str);
    if let Some(rt) = runtime {
        say!("    runtime: {}", rt);
    }
    say!("run `luxctltasks` to see available tasks");

    Ok(())
}

/// handle `luxctlproject status`
pub fn status() -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `luxctl auth --token $token`");
        return Ok(());
    }

    let state = ProjectState::load(config.expose_token())?;

    if let Some(project) = state.get_active() {
        say!("active project: {}", project.name);
        say!("          slug: {}", project.slug);
        say!("     workspace: {}", project.workspace);
        if let Some(ref rt) = project.runtime {
            say!("       runtime: {}", rt);
        } else {
            say!("       runtime: not set");
        }
        say!(
            "      progress: {}/{} tasks completed",
            project.completed_count(),
            project.tasks.len()
        );
        say!("run `luxctltasks` for task list");
    } else {
        say!("no active project");
        say!("run `luxctlproject start --slug <SLUG>` to start one");
    }

    Ok(())
}

/// handle `luxctlproject stop`
pub fn stop() -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `luxctl auth --token $token`");
        return Ok(());
    }

    let mut state = ProjectState::load(config.expose_token())?;

    if state.get_active().is_some() {
        let name = state
            .get_active()
            .map(|p| p.name.clone())
            .unwrap_or_default();
        state.clear_active();
        state.save(config.expose_token())?;
        say!("stopped working on: {}", name);
    } else {
        say!("no active project to stop");
    }

    Ok(())
}

/// handle `luxctlproject set --runtime <runtime>`
pub fn set_runtime(runtime: &str) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `luxctl auth --token $token`");
        return Ok(());
    }

    let mut state = ProjectState::load(config.expose_token())?;

    if state.get_active().is_some() {
        state.set_runtime(runtime);
        state.save(config.expose_token())?;
        cheer!("runtime set to: {}", runtime);
    } else {
        oops!("no active project");
        say!("run `luxctlproject start --slug <SLUG>` first");
    }

    Ok(())
}

/// handle `luxctlproject set --workspace <path>`
pub fn set_workspace(workspace: &str) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `luxctl auth --token $token`");
        return Ok(());
    }

    let mut state = ProjectState::load(config.expose_token())?;

    if state.get_active().is_none() {
        oops!("no active project");
        say!("run `luxctlproject start --slug <SLUG>` first");
        return Ok(());
    }

    // resolve to absolute path
    let workspace_path = std::path::Path::new(workspace);
    let absolute_workspace = if workspace_path.is_absolute() {
        workspace_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| color_eyre::eyre::eyre!("cannot get cwd: {}", e))?
            .join(workspace_path)
    };

    if !absolute_workspace.exists() {
        oops!("directory does not exist: {}", absolute_workspace.display());
        return Ok(());
    }

    // canonicalize to get clean path without . or ..
    let canonical = absolute_workspace
        .canonicalize()
        .map_err(|e| color_eyre::eyre::eyre!("cannot resolve path: {}", e))?;

    let workspace_str = canonical.to_string_lossy().to_string();
    state.set_workspace(&workspace_str);
    state.save(config.expose_token())?;
    cheer!("workspace set to: {}", workspace_str);

    Ok(())
}
