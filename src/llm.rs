use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::config::{Config, ProviderKind};
use crate::prompt::build_system_prompt;

#[derive(Serialize)]
struct OpenAiRequest {
    model: String,
    messages: Vec<OpenAiMessage>,
    temperature: f32,
}

#[derive(Serialize)]
struct OpenAiMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    choices: Vec<OpenAiChoice>,
}

#[derive(Deserialize)]
struct OpenAiChoice {
    message: OpenAiResponseMessage,
}

#[derive(Deserialize)]
struct OpenAiResponseMessage {
    content: Option<String>,
}

#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    max_tokens: u32,
    system: String,
    messages: Vec<AnthropicMessage>,
}

#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<AnthropicContentBlock>,
}

#[derive(Deserialize)]
struct AnthropicContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
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

    let content = match config.kind {
        ProviderKind::OpenAi => ask_openai(&client, config, &system_prompt, question).await?,
        ProviderKind::Anthropic => ask_anthropic(&client, config, &system_prompt, question).await?,
    };

    parse_response(&content)
}

async fn ask_openai(
    client: &Client,
    config: &Config,
    system_prompt: &str,
    question: &str,
) -> Result<String, String> {
    let request = OpenAiRequest {
        model: config.model.clone(),
        messages: vec![
            OpenAiMessage {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            OpenAiMessage {
                role: "user".to_string(),
                content: question.to_string(),
            },
        ],
        temperature: 0.0,
    };

    let response = client
        .post(format!("{}/chat/completions", config.base_url))
        .header("Authorization", format!("Bearer {}", config.api_key))
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Failed to call API: {e}"))?;

    if !response.status().is_success() {
        return Err(await_api_error(response, "API").await);
    }

    let chat_response: OpenAiResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse API response: {e}"))?;

    chat_response
        .choices
        .first()
        .and_then(|c| c.message.content.as_ref())
        .cloned()
        .ok_or_else(|| "No response from provider".to_string())
}

async fn ask_anthropic(
    client: &Client,
    config: &Config,
    system_prompt: &str,
    question: &str,
) -> Result<String, String> {
    let request = AnthropicRequest {
        model: config.model.clone(),
        max_tokens: 1024,
        system: system_prompt.to_string(),
        messages: vec![AnthropicMessage {
            role: "user".to_string(),
            content: question.to_string(),
        }],
    };

    let response = client
        .post(format!("{}/messages", config.base_url))
        .header("x-api-key", &config.api_key)
        .header("anthropic-version", "2023-06-01")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Failed to call Anthropic API: {e}"))?;

    if !response.status().is_success() {
        return Err(await_api_error(response, "Anthropic API").await);
    }

    let anthropic_response: AnthropicResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse Anthropic response: {e}"))?;

    extract_anthropic_text(&anthropic_response)
        .ok_or_else(|| "No text response from Anthropic".to_string())
}

fn extract_anthropic_text(response: &AnthropicResponse) -> Option<String> {
    response
        .content
        .iter()
        .find(|b| b.block_type == "text")
        .and_then(|b| b.text.as_ref())
        .cloned()
}

async fn await_api_error(response: reqwest::Response, label: &str) -> String {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    format!("{label} error ({status}): {body}")
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
#[expect(clippy::unwrap_used, reason = "test code")]
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

    #[test]
    fn extracts_first_text_block_from_anthropic_response() {
        let response: AnthropicResponse = serde_json::from_str(
            r#"{"content": [
                {"type": "text", "text": "{\"command\": \"ls\", \"explanation\": \"e\", \"risk\": \"safe\"}"},
                {"type": "text", "text": "ignored"}
            ]}"#,
        )
        .unwrap();
        let text = extract_anthropic_text(&response).unwrap();
        assert!(text.contains("\"command\": \"ls\""));
    }

    #[test]
    fn ignores_non_text_blocks() {
        let response: AnthropicResponse = serde_json::from_str(
            r#"{"content": [
                {"type": "tool_use", "text": null},
                {"type": "text", "text": "hello"}
            ]}"#,
        )
        .unwrap();
        assert_eq!(extract_anthropic_text(&response).unwrap(), "hello");
    }

    #[test]
    fn returns_none_when_no_text_block() {
        let response: AnthropicResponse =
            serde_json::from_str(r#"{"content": [{"type": "tool_use", "text": null}]}"#).unwrap();
        assert!(extract_anthropic_text(&response).is_none());
    }
}
