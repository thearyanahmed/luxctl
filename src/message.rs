use colored::Colorize;
use termimad::MadSkin;

use crate::api::{PaginatedResponse, Project, Task};
use crate::state::ActiveProject;
use crate::tasks::{TestCase, TestResults};

pub struct Message;

// All prefixes padded inside brackets to match "[ERROR]" (5 chars inside)

impl Message {
    pub fn greet(name: &str) {
        let msg = format!(
            "hello {}, welcome to {}!",
            name.bold(),
            "projectlighthouse".yellow()
        );
        println!("{} {}", "[LUX  ]".blue(), msg);
    }

    pub fn say(msg: &str) {
        println!("{} {}", "[LUX  ]".blue(), msg);
    }

    pub fn cheer(msg: &str) {
        println!("{} {}", "[OK   ]".green(), msg);
    }

    pub fn complain(msg: &str) {
        eprintln!("{} {}", "[WARN ]".yellow(), msg);
    }

    pub fn oops(msg: &str) {
        eprintln!("{} {}", "[ERROR]".red(), msg);
    }

    pub fn print_projects(response: &PaginatedResponse<Project>) {
        Self::say(&format!(
            "available projects ({} total):\n",
            response.meta.total
        ));

        for project in &response.data {
            Self::print_project(project);
        }
    }

    fn print_project(project: &Project) {
        println!("  {} {}", "[#]".dimmed(), project.name.bold());
        if let Some(desc) = &project.short_description {
            println!("    {}", desc);
        }
        let tasks_count = project.tasks_count.unwrap_or(0);
        println!(
            "    {} {}  {} {}",
            "slug:".dimmed(),
            project.slug.dimmed(),
            "tasks:".dimmed(),
            tasks_count.to_string().dimmed()
        );
        println!("    {} {}\n", "url:".dimmed(), project.url().dimmed());
    }

    pub fn print_project_detail(project: &Project) {
        println!("  {} {}", "[#]".dimmed(), project.name.bold());

        if let Some(desc) = &project.short_description {
            println!("    {}", desc);
        }

        println!("    {} {}", "slug:".dimmed(), project.slug.dimmed());
        println!("    {} {}", "url:".dimmed(), project.url().dimmed());

        println!();

        if let Some(tasks) = &project.tasks {
            println!("  {} ({}):\n", "tasks".bold(), tasks.len());

            let task_count = tasks.len();
            for (index, task) in tasks.iter().enumerate() {
                let is_last = index == task_count - 1;

                // Timeline connector
                let connector = if is_last { "└" } else { "├" };
                let line_char = if is_last { " " } else { "│" };

                // Show status marker only for completed tasks
                let is_completed = task.status == "completed" || task.status == "success";
                let status_marker = if is_completed { " ✓".green().to_string() } else { String::new() };

                println!(
                    "    {}── {} {}  {}",
                    connector.dimmed(),
                    task.title.bold(),
                    task.scores.dimmed(),
                    status_marker
                );

                // Add empty line between tasks (except after last)
                if !is_last {
                    println!("    {}", line_char.dimmed());
                }
            }

            // Show first task details at the end
            if let Some(first_task) = tasks.first() {
                println!();
                println!("  {} {}", "next up:".dimmed(), first_task.title.bold());
                println!();

                let skin = MadSkin::default();
                let rendered = format!("{}", skin.text(&first_task.description, None));
                for line in rendered.lines() {
                    println!("    {}", line);
                }
            }
        }
    }

    pub fn print_task_header(task: &Task, detailed: bool) {
        println!("{} {}", "[TASK ]".blue(), task.title.bold());

        if detailed {
            let skin = MadSkin::default();
            let rendered = format!("{}", skin.text(&task.description, None));
            for line in rendered.lines() {
                println!("    {}", line);
            }
        }
    }

    pub fn print_validators_start(count: usize) {
        println!(
            "{} running {} validator{}...",
            "[RUN  ]".blue(),
            count,
            if count == 1 { "" } else { "s" }
        );
    }

    pub fn print_test_case(test: &TestCase, index: usize) {
        let status_str = if test.passed() {
            "[PASS ]".green()
        } else {
            "[FAIL ]".red()
        };

        println!(
            "{} {} {}",
            status_str,
            format!("#{}", index + 1).dimmed(),
            test.name
        );

        // show message on failure
        if !test.passed() {
            // 7 chars bracket + 1 space = 8 chars indent
            println!("{:8}{}", "", test.message().red());
        }
    }

    pub fn print_test_results(results: &TestResults) {
        if results.all_passed() {
            println!(
                "{} all {} tests passed!",
                "[OK   ]".green(),
                results.total()
            );
        } else {
            println!(
                "{} {}/{} tests passed",
                "[FAIL ]".red(),
                results.passed(),
                results.total()
            );
        }
    }

    pub fn print_connection_error(port: u16) {
        Self::oops(&format!(
            "could not connect to server on port {}",
            port
        ));
        println!();
        println!("    make sure your server is running:");
        println!("    {}", "  ./your-server".dimmed());
        println!();
    }

    /// print task list for active project
    pub fn print_task_list(project: &ActiveProject) {
        Self::say(&format!("tasks for: {}\n", project.name.bold()));

        // table header
        println!(
            "  {}  {}  {}  {}",
            "#".dimmed(),
            "Status".dimmed(),
            format!("{:30}", "Task").dimmed(),
            "Points".dimmed()
        );

        // compute earned points
        let mut earned = 0;

        for (i, task) in project.tasks.iter().enumerate() {
            let (status, status_color) = match task.status.as_str() {
                "challenge_completed" => {
                    earned += task.points;
                    ("[DONE]".to_string(), "green")
                }
                "challenge_failed" => ("[FAIL]".to_string(), "red"),
                "challenged" => ("[....]".to_string(), "yellow"),
                _ => ("[    ]".to_string(), "white"),
            };

            let status_display = match status_color {
                "green" => status.green().to_string(),
                "red" => status.red().to_string(),
                "yellow" => status.yellow().to_string(),
                _ => status.dimmed().to_string(),
            };

            let index = (i + 1).to_string();
            let title_padded = format!("{:30}", task.title);
            println!(
                "  {}  {}  {}  {} XP",
                index.dimmed(),
                status_display,
                title_padded,
                task.points
            );
        }

        println!();
        println!(
            "       progress: {}/{} completed | {}/{} XP earned",
            project.completed_count(),
            project.tasks.len(),
            earned,
            project.total_points()
        );
    }
}

/// Welcome/greet the user
#[macro_export]
macro_rules! greet {
    ($name:expr) => {
        $crate::message::Message::greet($name)
    };
}

/// General info message
#[macro_export]
macro_rules! say {
    ($msg:expr) => {
        $crate::message::Message::say($msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::message::Message::say(&format!($fmt, $($arg)*))
    };
}

/// Success message
#[macro_export]
macro_rules! cheer {
    ($msg:expr) => {
        $crate::message::Message::cheer($msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::message::Message::cheer(&format!($fmt, $($arg)*))
    };
}

/// Warning message
#[macro_export]
macro_rules! complain {
    ($msg:expr) => {
        $crate::message::Message::complain($msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::message::Message::complain(&format!($fmt, $($arg)*))
    };
}

/// Error message
#[macro_export]
macro_rules! oops {
    ($msg:expr) => {
        $crate::message::Message::oops($msg)
    };
    ($fmt:expr, $($arg:tt)*) => {
        $crate::message::Message::oops(&format!($fmt, $($arg)*))
    };
}
