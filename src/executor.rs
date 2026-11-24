use std::io::{self, Write};
use std::process::Command;

use colored::Colorize;

use crate::llm::ShellCommand;

/// Program and flag that commands run through. The system prompt reports the same values.
pub const SHELL: (&str, &str) = if cfg!(windows) {
    ("cmd", "/C")
} else {
    ("sh", "-c")
};

/// Runs the command and returns the exit code shelly should exit with.
pub fn execute(shell_cmd: &ShellCommand, dry_run: bool) -> Result<i32, String> {
    if shell_cmd.command.is_empty() {
        println!("{}", shell_cmd.explanation.yellow());
        return Ok(0);
    }

    // Show the command and explanation prominently
    println!();
    println!(
        "  {} {}",
        "Command:".cyan().bold(),
        shell_cmd.command.white().bold()
    );
    println!("  {} {}", "Reason: ".cyan(), shell_cmd.explanation.dimmed());
    println!();

    if dry_run {
        println!("{}", "Dry run: command not executed.".dimmed());
        return Ok(0);
    }

    // If risky, ask for confirmation
    if shell_cmd.is_risky() {
        println!(
            "{}",
            "⚠ This command is potentially destructive.".yellow().bold()
        );
        print!("Run it? [y/N] ");
        io::stdout()
            .flush()
            .map_err(|e| format!("Failed to flush stdout: {e}"))?;

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Failed to read input: {e}"))?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("{}", "Cancelled.".dimmed());
            return Ok(0);
        }
    }

    // Execute the command
    let (program, flag) = SHELL;
    let status = Command::new(program)
        .arg(flag)
        .arg(&shell_cmd.command)
        .status()
        .map_err(|e| format!("Failed to execute command: {e}"))?;

    // No code means the command was killed by a signal; report a generic failure.
    let code = status.code().unwrap_or(1);
    if code != 0 {
        println!("\n{}", format!("Command exited with code {code}").red());
    }

    Ok(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn safe(command: &str) -> ShellCommand {
        ShellCommand {
            command: command.to_string(),
            explanation: String::new(),
            risk: "safe".to_string(),
        }
    }

    #[test]
    fn returns_command_exit_code() {
        assert_eq!(execute(&safe("exit 0"), false), Ok(0));
        assert_eq!(execute(&safe("exit 3"), false), Ok(3));
    }

    #[test]
    #[cfg(unix)]
    fn signal_death_is_a_failure() {
        assert_eq!(execute(&safe("kill -KILL $$"), false), Ok(1));
    }

    #[test]
    fn skipped_commands_succeed() {
        assert_eq!(execute(&safe("exit 3"), true), Ok(0));
        assert_eq!(execute(&safe(""), false), Ok(0));
    }
}
