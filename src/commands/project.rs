use color_eyre::eyre::Result;

use crate::api::LighthouseAPIClient;
use crate::config::Config;
use crate::state::ProjectState;
use crate::ui::UI;

/// handle `luxctl project start --slug <slug> --workspace <path> [--runtime <runtime>]`
pub async fn start(slug: &str, workspace: &str, runtime: Option<&str>) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error("not authenticated", Some("run `luxctl auth --token $token`"));
        return Ok(());
    }

    let client = LighthouseAPIClient::from_config(&config);

    let project = match client.project_by_slug(slug).await {
        Ok(p) => p,
        Err(err) => {
            UI::error(&format!("project '{}' not found", slug), Some(&format!("{}", err)));
            UI::note("run `luxctl projects` to see available projects");
            return Ok(());
        }
    };

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

    let mut state = ProjectState::load(config.expose_token())?;
    state.set_active(&project.slug, &project.name, tasks, &workspace_str, runtime);
    state.save(config.expose_token())?;

    UI::success(&format!("now working on: {}", project.name));
    UI::kv("workspace", &workspace_str);
    if let Some(rt) = runtime {
        UI::kv("runtime", rt);
    }
    UI::note("run `luxctl tasks` to see available tasks");

    Ok(())
}

/// handle `luxctl project status`
pub fn status() -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error("not authenticated", Some("run `luxctl auth --token $token`"));
        return Ok(());
    }

    let state = ProjectState::load(config.expose_token())?;

    if let Some(project) = state.get_active() {
        UI::kv_aligned("active project", &project.name, 14);
        UI::kv_aligned("slug", &project.slug, 14);
        UI::kv_aligned("workspace", &project.workspace, 14);
        if let Some(ref rt) = project.runtime {
            UI::kv_aligned("runtime", rt, 14);
        } else {
            UI::kv_aligned("runtime", "not set", 14);
        }
        UI::kv_aligned(
            "progress",
            &format!("{}/{} tasks completed", project.completed_count(), project.tasks.len()),
            14,
        );
        UI::note("run `luxctl tasks` for task list");
    } else {
        UI::info("no active project");
        UI::note("run `luxctl project start --slug <SLUG>` to start one");
    }

    Ok(())
}

/// handle `luxctl project stop`
pub fn stop() -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error("not authenticated", Some("run `luxctl auth --token $token`"));
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
        UI::success(&format!("stopped working on: {}", name));
    } else {
        UI::info("no active project to stop");
    }

    Ok(())
}

/// handle `luxctl project set --runtime <runtime>`
pub fn set_runtime(runtime: &str) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error("not authenticated", Some("run `luxctl auth --token $token`"));
        return Ok(());
    }

    let mut state = ProjectState::load(config.expose_token())?;

    if state.get_active().is_some() {
        state.set_runtime(runtime);
        state.save(config.expose_token())?;
        UI::success(&format!("runtime set to: {}", runtime));
    } else {
        UI::error("no active project", None);
        UI::note("run `luxctl project start --slug <SLUG>` first");
    }

    Ok(())
}

/// handle `luxctl project set --workspace <path>`
pub fn set_workspace(workspace: &str) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error("not authenticated", Some("run `luxctl auth --token $token`"));
        return Ok(());
    }

    let mut state = ProjectState::load(config.expose_token())?;

    if state.get_active().is_none() {
        UI::error("no active project", None);
        UI::note("run `luxctl project start --slug <SLUG>` first");
        return Ok(());
    }

    let workspace_path = std::path::Path::new(workspace);
    let absolute_workspace = if workspace_path.is_absolute() {
        workspace_path.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| color_eyre::eyre::eyre!("cannot get cwd: {}", e))?
            .join(workspace_path)
    };

    if !absolute_workspace.exists() {
        UI::error(
            "directory does not exist",
            Some(&absolute_workspace.to_string_lossy()),
        );
        return Ok(());
    }

    let canonical = absolute_workspace
        .canonicalize()
        .map_err(|e| color_eyre::eyre::eyre!("cannot resolve path: {}", e))?;

    let workspace_str = canonical.to_string_lossy().to_string();
    state.set_workspace(&workspace_str);
    state.save(config.expose_token())?;
    UI::success(&format!("workspace set to: {}", workspace_str));

    Ok(())
}
