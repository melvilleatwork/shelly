use std::io::{self, Write};
use std::process::Command;

use colored::Colorize;

use crate::llm::ShellCommand;

pub fn execute(shell_cmd: &ShellCommand) -> Result<(), String> {
    if shell_cmd.command.is_empty() {
        println!("{}", shell_cmd.explanation.yellow());
        return Ok(());
    }

    // Show the command and explanation prominently
    println!();
    println!("  {} {}", "Command:".cyan().bold(), shell_cmd.command.white().bold());
    println!("  {} {}", "Reason: ".cyan(), shell_cmd.explanation.dimmed());
    println!();

    // If risky, ask for confirmation
    if shell_cmd.is_risky() {
        println!(
            "{}",
            "⚠ This command is potentially destructive.".yellow().bold()
        );
        print!("Run it? [y/N] ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin()
            .read_line(&mut input)
            .map_err(|e| format!("Failed to read input: {e}"))?;

        if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
            println!("{}", "Cancelled.".dimmed());
            return Ok(());
        }
    }

    // Execute the command
    let shell = if cfg!(windows) { "cmd" } else { "sh" };
    let flag = if cfg!(windows) { "/C" } else { "-c" };

    let status = Command::new(shell)
        .arg(flag)
        .arg(&shell_cmd.command)
        .status()
        .map_err(|e| format!("Failed to execute command: {e}"))?;

    if !status.success() {
        let code = status.code().unwrap_or(-1);
        println!(
            "\n{}",
            format!("Command exited with code {code}").red()
        );
    }

    Ok(())
}
