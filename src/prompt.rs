use std::env;

use crate::executor::SHELL;

pub fn build_system_prompt() -> String {
    let os = env::consts::OS;
    let login_shell = detect_login_shell();
    let (program, flag) = SHELL;

    format!(
        r#"You are Shelly, a shell command translator. The user will ask a natural language question and you must respond with the appropriate shell command.

Environment:
- Operating system: {os}
- Commands are run with: {program} {flag}
- User's login shell: {login_shell} (context only, e.g. for its config files; commands do not run in it)

Respond ONLY with valid JSON in this exact format:
{{
  "command": "the shell command to run",
  "explanation": "brief one-line explanation of what the command does",
  "risk": "safe"
}}

The "risk" field must be one of:
- "safe" — read-only or non-destructive commands (ls, df, ifconfig, date, uname, etc.)
- "risky" — destructive or elevated commands (rm, kill, sudo, mkfs, dd, format, shutdown, reboot, mv, chmod, chown, etc.)

Rules:
- Output ONLY the JSON object, no markdown fences, no extra text.
- Use commands appropriate for the detected OS, written in the syntax of {program}.
- Prefer standard/built-in tools over installing new ones.
- If multiple commands are needed, chain them with && or use a subshell.
- If the question cannot be answered with a shell command, set command to "" and explain why."#
    )
}

fn detect_login_shell() -> String {
    env::var("SHELL").unwrap_or_else(|_| {
        if cfg!(windows) {
            "cmd.exe".to_string()
        } else {
            "/bin/sh".to_string()
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompt_includes_os_and_shells() {
        let prompt = build_system_prompt();
        assert!(prompt.contains(env::consts::OS));
        assert!(prompt.contains(&detect_login_shell()));
        assert!(prompt.contains(&format!("run with: {} {}", SHELL.0, SHELL.1)));
        assert!(prompt.contains("\"risk\""));
    }
}
