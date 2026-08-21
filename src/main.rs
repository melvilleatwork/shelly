mod config;
mod executor;
mod llm;
mod prompt;

use clap::Parser;
use colored::Colorize;

#[derive(Parser)]
#[command(
    name = "shelly",
    about = "Translate natural language into shell commands",
    version
)]
struct Cli {
    /// The question to translate into a shell command
    question: String,

    /// Show the command without executing it
    #[arg(short, long)]
    dry_run: bool,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let config = match config::Config::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} {e}", "Error:".red().bold());
            std::process::exit(1);
        }
    };

    println!("{}", "Thinking...".dimmed());

    match llm::ask_llm(&config, &cli.question).await {
        Ok(shell_cmd) => {
            // Clear the "Thinking..." line
            print!("\x1b[1A\x1b[2K");
            if let Err(e) = executor::execute(&shell_cmd, cli.dry_run) {
                eprintln!("{} {e}", "Error:".red().bold());
                std::process::exit(1);
            }
        }
        Err(e) => {
            eprintln!("{} {e}", "Error:".red().bold());
            std::process::exit(1);
        }
    }
}
