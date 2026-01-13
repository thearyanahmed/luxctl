use chrono::{DateTime, Utc};
use color_eyre::eyre::{self, Ok};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{fs, path::PathBuf};

use crate::api::Task;

static CFG_DIR: &str = ".lux";
static STATE_FILE: &str = "state.json";

// salt used for HMAC key derivation (combined with user token)
static HMAC_SALT: &str = "lux-state-integrity-v1";

type HmacSha256 = Hmac<Sha256>;

/// task data cached for offline access and integrity protection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedTask {
    pub id: i32,
    pub slug: String,
    pub title: String,
    pub points: i32,
    pub status: String,
    pub sort_order: i32,
    pub validators: Vec<String>,
}

impl CachedTask {
    /// create from API task, extracting base points from scores string
    pub fn from_api_task(task: &Task) -> Self {
        // scores format: "attempts:minutes:points|..." - take max points from first tier
        let points = task
            .scores
            .split('|')
            .next()
            .and_then(|tier| tier.split(':').nth(2))
            .and_then(|p| p.parse().ok())
            .unwrap_or(0);

        CachedTask {
            id: task.id,
            slug: task.slug.clone(),
            title: task.title.clone(),
            points,
            status: task.status.clone(),
            sort_order: task.sort_order,
            validators: task.validators.clone(),
        }
    }
}

/// active project with cached task data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActiveProject {
    pub slug: String,
    pub name: String,
    pub fetched_at: DateTime<Utc>,
    pub tasks: Vec<CachedTask>,
}

impl ActiveProject {
    pub fn total_points(&self) -> i32 {
        self.tasks.iter().map(|t| t.points).sum()
    }

    pub fn completed_count(&self) -> usize {
        self.tasks
            .iter()
            .filter(|t| t.status == "challenge_completed")
            .count()
    }
}

/// internal state file format (includes checksum)
#[derive(Debug, Serialize, Deserialize)]
struct StateFile {
    active_project: Option<ActiveProject>,
    checksum: String,
}

/// project state manager with tamper detection
#[derive(Debug)]
pub struct ProjectState {
    pub active_project: Option<ActiveProject>,
}

impl ProjectState {
    /// create empty state
    pub fn new() -> Self {
        ProjectState {
            active_project: None,
        }
    }

    /// load state from disk, verifying integrity with HMAC
    /// if checksum fails, returns empty state (forces re-fetch)
    pub fn load(token: &str) -> eyre::Result<Self> {
        let path = Self::state_path()?;

        if !path.exists() {
            return Ok(ProjectState::new());
        }

        let content = fs::read_to_string(&path)
            .map_err(|e| eyre::eyre!("failed to read state file: {}", e))?;

        let state_file: StateFile = serde_json::from_str(&content)
            .map_err(|e| eyre::eyre!("failed to parse state file: {}", e))?;

        // verify checksum
        let expected = Self::compute_checksum(&state_file.active_project, token);
        if state_file.checksum != expected {
            log::warn!("state file checksum mismatch, clearing state");
            // tampered or token changed - clear state
            let empty = ProjectState::new();
            empty.save(token)?;
            return Ok(empty);
        }

        Ok(ProjectState {
            active_project: state_file.active_project,
        })
    }

    /// save state to disk with HMAC checksum
    pub fn save(&self, token: &str) -> eyre::Result<()> {
        let path = Self::state_path()?;

        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }

        let checksum = Self::compute_checksum(&self.active_project, token);
        let state_file = StateFile {
            active_project: self.active_project.clone(),
            checksum,
        };

        let content = serde_json::to_string_pretty(&state_file)
            .map_err(|e| eyre::eyre!("failed to serialize state: {}", e))?;

        fs::write(&path, content)?;
        log::debug!("state saved to {}", path.display());

        Ok(())
    }

    /// set active project from API data
    pub fn set_active(&mut self, slug: &str, name: &str, tasks: &[Task]) {
        let cached_tasks: Vec<CachedTask> = tasks.iter().map(CachedTask::from_api_task).collect();

        self.active_project = Some(ActiveProject {
            slug: slug.to_string(),
            name: name.to_string(),
            fetched_at: Utc::now(),
            tasks: cached_tasks,
        });
    }

    /// clear active project
    pub fn clear_active(&mut self) {
        self.active_project = None;
    }

    /// get reference to active project
    pub fn get_active(&self) -> Option<&ActiveProject> {
        self.active_project.as_ref()
    }

    /// update cached tasks (for refresh)
    pub fn refresh_tasks(&mut self, tasks: &[Task]) {
        if let Some(ref mut project) = self.active_project {
            project.tasks = tasks.iter().map(CachedTask::from_api_task).collect();
            project.fetched_at = Utc::now();
        }
    }

    /// update a single task's status (e.g., after successful submission)
    pub fn update_task_status(&mut self, task_id: i32, new_status: &str) {
        if let Some(ref mut project) = self.active_project {
            if let Some(task) = project.tasks.iter_mut().find(|t| t.id == task_id) {
                task.status = new_status.to_string();
            }
        }
    }

    /// compute HMAC-SHA256 checksum of project data
    /// returns empty string if HMAC creation fails (should never happen for SHA256)
    fn compute_checksum(project: &Option<ActiveProject>, token: &str) -> String {
        // derive key from token + salt
        let key = format!("{}{}", token, HMAC_SALT);

        // HMAC-SHA256 accepts any key length, so this should never fail
        let Some(mut mac) = HmacSha256::new_from_slice(key.as_bytes()).ok() else {
            log::error!("failed to create HMAC - this should never happen");
            return String::new();
        };

        // hash the project data as JSON
        let data = serde_json::to_string(project).unwrap_or_default();
        mac.update(data.as_bytes());

        let result = mac.finalize();
        hex::encode(result.into_bytes())
    }

    fn state_path() -> eyre::Result<PathBuf> {
        let home =
            dirs::home_dir().ok_or_else(|| eyre::eyre!("could not determine home directory"))?;

        Ok(home.join(CFG_DIR).join(STATE_FILE))
    }
}

impl Default for ProjectState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_token() -> &'static str {
        "test-secret-token-123"
    }

    #[test]
    fn test_cached_task_from_api_task() {
        let api_task = Task {
            id: 1,
            slug: "test-task".to_string(),
            title: "Test Task".to_string(),
            description: "Description".to_string(),
            sort_order: 1,
            scores: "5:10:50|10:20:35".to_string(),
            status: "challenge_awaits".to_string(),
            abandoned_deduction: 5,
            hints: vec![],
            validators: vec!["tcp_listening:int(8080)".to_string()],
        };

        let cached = CachedTask::from_api_task(&api_task);

        assert_eq!(cached.id, 1);
        assert_eq!(cached.slug, "test-task");
        assert_eq!(cached.points, 50); // max points from first tier
        assert_eq!(cached.validators.len(), 1);
    }

    #[test]
    fn test_compute_checksum_deterministic() {
        let project = Some(ActiveProject {
            slug: "test".to_string(),
            name: "Test Project".to_string(),
            fetched_at: DateTime::parse_from_rfc3339("2026-01-01T00:00:00Z")
                .expect("valid date")
                .with_timezone(&Utc),
            tasks: vec![],
        });

        let checksum1 = ProjectState::compute_checksum(&project, test_token());
        let checksum2 = ProjectState::compute_checksum(&project, test_token());

        assert_eq!(checksum1, checksum2);
    }

    #[test]
    fn test_checksum_changes_with_data() {
        let project1 = Some(ActiveProject {
            slug: "test1".to_string(),
            name: "Test Project".to_string(),
            fetched_at: Utc::now(),
            tasks: vec![],
        });

        let project2 = Some(ActiveProject {
            slug: "test2".to_string(),
            name: "Test Project".to_string(),
            fetched_at: Utc::now(),
            tasks: vec![],
        });

        let checksum1 = ProjectState::compute_checksum(&project1, test_token());
        let checksum2 = ProjectState::compute_checksum(&project2, test_token());

        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn test_checksum_changes_with_token() {
        let project = Some(ActiveProject {
            slug: "test".to_string(),
            name: "Test Project".to_string(),
            fetched_at: Utc::now(),
            tasks: vec![],
        });

        let checksum1 = ProjectState::compute_checksum(&project, "token1");
        let checksum2 = ProjectState::compute_checksum(&project, "token2");

        assert_ne!(checksum1, checksum2);
    }

    #[test]
    fn test_active_project_stats() {
        let project = ActiveProject {
            slug: "test".to_string(),
            name: "Test".to_string(),
            fetched_at: Utc::now(),
            tasks: vec![
                CachedTask {
                    id: 1,
                    slug: "t1".to_string(),
                    title: "Task 1".to_string(),
                    points: 25,
                    status: "challenge_completed".to_string(),
                    sort_order: 1,
                    validators: vec![],
                },
                CachedTask {
                    id: 2,
                    slug: "t2".to_string(),
                    title: "Task 2".to_string(),
                    points: 50,
                    status: "challenge_awaits".to_string(),
                    sort_order: 2,
                    validators: vec![],
                },
            ],
        };

        assert_eq!(project.total_points(), 75);
        assert_eq!(project.completed_count(), 1);
    }
}
