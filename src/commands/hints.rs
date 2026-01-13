use color_eyre::eyre::Result;
use colored::Colorize;

use crate::api::LighthouseAPIClient;
use crate::config::Config;
use crate::{oops, say};

/// handle `lux hints --task <slug>`
pub async fn list(task_slug: &str) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `lux auth --token <TOKEN>`");
        return Ok(());
    }

    let client = LighthouseAPIClient::from_config(&config);

    let response = match client.hints(task_slug).await {
        Ok(r) => r,
        Err(err) => {
            oops!("failed to fetch hints: {}", err);
            return Ok(());
        }
    };

    if response.data.is_empty() {
        say!("no hints available for task '{}'", task_slug);
        return Ok(());
    }

    say!("hints for task: {}\n", task_slug);

    for (i, hint) in response.data.iter().enumerate() {
        let status = if hint.is_unlocked {
            "[UNLOCKED]".green()
        } else if hint.is_available {
            "[AVAILABLE]".yellow()
        } else {
            "[LOCKED]".dimmed()
        };

        println!(
            "  {}  {} {}",
            format!("#{}", i + 1).dimmed(),
            status,
            format!("-{} XP", hint.points_deduction).dimmed()
        );

        if hint.is_unlocked {
            if let Some(text) = &hint.text {
                println!("       {}", text);
            }
        } else if hint.is_available {
            println!(
                "       {} lux hint unlock --task {} --hint {}",
                "unlock:".dimmed(),
                task_slug,
                hint.uuid
            );
        } else {
            println!("       {}", "not yet available".dimmed());
        }
        println!();
    }

    Ok(())
}

/// handle `lux hint unlock --task <slug> --hint <uuid>`
pub async fn unlock(task_slug: &str, hint_uuid: &str) -> Result<()> {
    let config = Config::load()?;
    if !config.has_auth_token() {
        oops!("not authenticated. Run: `lux auth --token <TOKEN>`");
        return Ok(());
    }

    let client = LighthouseAPIClient::from_config(&config);

    let response = match client.unlock_hint(task_slug, hint_uuid).await {
        Ok(r) => r,
        Err(err) => {
            oops!("failed to unlock hint: {}", err);
            return Ok(());
        }
    };

    say!("{}", response.message);
    say!("points deducted: -{}", response.points_deducted);
    println!();
    println!("  {}", "hint:".bold());
    println!("  {}", response.data.text);

    Ok(())
}
