use colored::Colorize;

use crate::VERSION;

const SYM_STEP: &str = "▸";
const SYM_PASS: &str = "✓";
const SYM_FAIL: &str = "✗";
const INDENT: &str = "  ";

/// UI output for running validators, matching HeroTerminal visual style
pub struct RunUI {
    task_name: String,
    total_validators: usize,
}

impl RunUI {
    pub fn new(task_name: &str, validator_count: usize) -> Self {
        Self {
            task_name: task_name.to_string(),
            total_validators: validator_count,
        }
    }

    /// print version header: "projectlighthouse CLI v1.2.0"
    pub fn header(&self) {
        println!("{}projectlighthouse CLI v{}", INDENT, VERSION.dimmed());
    }

    /// print progress step: "▸ Compiling project..."
    pub fn step(&self, msg: &str) {
        println!("{}{} {}", INDENT, SYM_STEP.blue(), msg);
    }

    pub fn blank_line(&self) {
        println!();
    }

    /// print passing test: "✓ server listening on port 4221"
    pub fn test_pass(&self, name: &str) {
        println!("{}{} {}", INDENT, SYM_PASS.green(), name);
    }

    /// print failing test with optional detail
    pub fn test_fail(&self, name: &str, detail: Option<&str>) {
        println!("{}{} {}", INDENT, SYM_FAIL.red(), name.red());

        if let Some(d) = detail {
            if !d.is_empty() && d != name {
                // truncate long error messages
                let display = if d.len() > 600 {
                    format!("{}...", &d[..600])
                } else {
                    d.to_string()
                };

                for line in display.lines() {
                    println!("{}  {}", INDENT, line.red());
                }
            }
        }
    }

    /// print success summary: "PASSED  All 3 tests passed!"
    pub fn summary_pass(&self, total: usize) {
        println!(
            "{}{}  All {} tests passed!",
            INDENT,
            "PASSED".green().bold(),
            total
        );
    }

    /// print failure summary: "FAILED  1 of 3 tests failed"
    pub fn summary_fail(&self, passed: usize, total: usize) {
        let failed = total - passed;
        println!(
            "{}{}  {} of {} tests failed",
            INDENT,
            "FAILED".red().bold(),
            failed,
            total
        );
    }

    /// print hint: "Hint: Check that your response includes the comma."
    pub fn hint(&self, text: &str) {
        println!();
        println!("{}{} {}", INDENT, "Hint:".dimmed(), text);
    }

    /// print task separator for multi-task validation
    pub fn task_separator(&self, current: usize, total: usize, task_slug: &str) {
        println!(
            "{}━━━ Task {}/{}: {} ━━━",
            INDENT,
            current,
            total,
            task_slug.bold()
        );
    }

    /// print points earned on success
    pub fn points_earned(&self, points: i32) {
        if points > 0 {
            println!("{}{}", INDENT, format!("+{} XP", points).green().bold());
        }
    }

    /// accessor for task name
    pub fn task_name(&self) -> &str {
        &self.task_name
    }

    /// accessor for validator count
    pub fn validator_count(&self) -> usize {
        self.total_validators
    }
}
