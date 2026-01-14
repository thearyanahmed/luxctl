use color_eyre::eyre::Result;

use crate::api::LighthouseAPIClient;
use crate::config::Config;
use crate::message::Message;
use crate::state::ProjectState;
use crate::{oops, say};

/// handle `lux task --task <slug|number> [--detailed]`
pub async fn show(task_id: &str, detailed: bool) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `lux auth --token <TOKEN>`");
        return Ok(());
    }

    let state = ProjectState::load(config.expose_token())?;
    let client = LighthouseAPIClient::from_config(&config);

    // get project slug from active project
    let project_slug = if let Some(p) = state.get_active() {
        p.slug.clone()
    } else {
        oops!("no active project");
        say!("run `lux project start --slug <SLUG>` first");
        return Ok(());
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
        if task_num == 0 || task_num > tasks.len() {
            oops!(
                "task #{} not found. valid range: 1-{}",
                task_num,
                tasks.len()
            );
            return Ok(());
        }
        &tasks[task_num - 1]
    } else if let Some(t) = tasks.iter().find(|t| t.slug == task_id) {
        t
    } else {
        oops!("task '{}' not found in project '{}'", task_id, project_slug);
        return Ok(());
    };

    Message::print_task_detail(task_data, detailed);

    Ok(())
}
