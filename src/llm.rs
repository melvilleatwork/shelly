use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::prompt::build_system_prompt;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
}

#[derive(Serialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct ShellCommand {
    pub command: String,
    pub explanation: String,
    pub risk: String,
}

impl ShellCommand {
    pub fn is_risky(&self) -> bool {
        self.risk.to_lowercase() == "risky"
    }
}

pub async fn ask_llm(config: &Config, question: &str) -> Result<ShellCommand, String> {
    let client = Client::new();
    let system_prompt = build_system_prompt();

    let request = ChatRequest {
        model: config.model.clone(),
        messages: vec![
            Message {
                role: "system".to_string(),
                content: system_prompt,
            },
            Message {
                role: "user".to_string(),
                content: question.to_string(),
            },
        ],
        temperature: 0.0,
    };

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Failed to call OpenAI API: {e}"))?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(format!("OpenAI API error ({status}): {body}"));
    }

    let chat_response: ChatResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse OpenAI response: {e}"))?;

    let content = chat_response
        .choices
        .first()
        .and_then(|c| c.message.content.as_ref())
        .ok_or("No response from OpenAI")?;

    parse_response(content)
}

pub fn parse_response(content: &str) -> Result<ShellCommand, String> {
    // Strip markdown fences if present
    let clean = content
        .trim()
        .strip_prefix("```json")
        .or_else(|| content.trim().strip_prefix("```"))
        .unwrap_or(content.trim());
    let clean = clean.strip_suffix("```").unwrap_or(clean).trim();

    let shell_cmd: ShellCommand = serde_json::from_str(clean).map_err(|e| {
        format!("Failed to parse LLM response as JSON: {e}\nRaw response: {content}")
    })?;

    Ok(shell_cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_json() {
        let cmd =
            parse_response(r#"{"command": "ls -la", "explanation": "list files", "risk": "safe"}"#)
                .unwrap();
        assert_eq!(cmd.command, "ls -la");
        assert_eq!(cmd.explanation, "list files");
        assert!(!cmd.is_risky());
    }

    #[test]
    fn parses_fenced_json() {
        let cmd = parse_response(
            "```json\n{\"command\": \"rm -rf /tmp/x\", \"explanation\": \"delete\", \"risk\": \"risky\"}\n```",
        )
        .unwrap();
        assert_eq!(cmd.command, "rm -rf /tmp/x");
        assert!(cmd.is_risky());
    }

    #[test]
    fn parses_fenced_without_language_tag() {
        let cmd = parse_response(
            "```\n{\"command\": \"df -h\", \"explanation\": \"disk usage\", \"risk\": \"safe\"}\n```",
        )
        .unwrap();
        assert_eq!(cmd.command, "df -h");
    }

    #[test]
    fn rejects_invalid_json() {
        assert!(parse_response("not json at all").is_err());
    }

    #[test]
    fn rejects_missing_fields() {
        assert!(parse_response(r#"{"command": "ls"}"#).is_err());
    }

    #[test]
    fn empty_command_is_not_risky() {
        let cmd =
            parse_response(r#"{"command": "", "explanation": "cannot be done", "risk": "safe"}"#)
                .unwrap();
        assert_eq!(cmd.command, "");
        assert!(!cmd.is_risky());
    }
}
