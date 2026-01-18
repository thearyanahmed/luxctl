use color_eyre::eyre::Result;

use crate::api::LighthouseAPIClient;
use crate::config::Config;
use crate::message::Message;
use crate::state::ProjectState;
use crate::ui::UI;

/// handle `luxctl tasks [--refresh]`
pub async fn list(refresh: bool) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        UI::error(
            "not authenticated",
            Some("run `luxctl auth --token $token`"),
        );
        return Ok(());
    }

    let mut state = ProjectState::load(config.expose_token())?;

    let project = if let Some(p) = state.get_active() {
        p.clone()
    } else {
        UI::error("no active project", None);
        UI::note("run `luxctl project start --slug <SLUG>` first");
        return Ok(());
    };

    // refresh from API if requested or no cached tasks
    if refresh || project.tasks.is_empty() {
        let client = LighthouseAPIClient::from_config(&config);

        let fresh_project = match client.project_by_slug(&project.slug).await {
            Ok(p) => p,
            Err(err) => {
                UI::error(
                    &format!("failed to fetch project '{}'", project.slug),
                    Some(&format!("{}", err)),
                );
                return Ok(());
            }
        };

        if let Some(tasks) = &fresh_project.tasks {
            state.refresh_tasks(tasks);
            state.save(config.expose_token())?;
        }
    }

    // print task list
    if let Some(active) = state.get_active() {
        Message::print_task_list(active);
    }

    Ok(())
}
