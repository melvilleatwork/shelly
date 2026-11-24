use reqwest::{Client, RequestBuilder, StatusCode};
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
    /// Fails closed: anything the model does not explicitly label "safe" needs confirmation.
    pub fn is_risky(&self) -> bool {
        !self.risk.eq_ignore_ascii_case("safe")
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

/// Adds the provider's auth header, or none when no key is configured.
fn with_auth(request: RequestBuilder, config: &Config) -> RequestBuilder {
    match (&config.api_key, config.kind) {
        (None, _) => request,
        (Some(key), ProviderKind::OpenAi) => {
            request.header("Authorization", format!("Bearer {key}"))
        }
        (Some(key), ProviderKind::Anthropic) => request.header("x-api-key", key),
    }
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

    let response = with_auth(
        client.post(format!("{}/chat/completions", config.base_url)),
        config,
    )
    .json(&request)
    .send()
    .await
    .map_err(|e| format!("Failed to call API: {e}"))?;

    if !response.status().is_success() {
        return Err(await_api_error(response, "API", config).await);
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

    let response = with_auth(client.post(format!("{}/messages", config.base_url)), config)
        .header("anthropic-version", "2023-06-01")
        .json(&request)
        .send()
        .await
        .map_err(|e| format!("Failed to call Anthropic API: {e}"))?;

    if !response.status().is_success() {
        return Err(await_api_error(response, "Anthropic API", config).await);
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

async fn await_api_error(response: reqwest::Response, label: &str, config: &Config) -> String {
    let status = response.status();
    let body = response.text().await.unwrap_or_default();
    let hint = missing_key_hint(status, config).unwrap_or_default();
    format!("{label} error ({status}): {body}{hint}")
}

/// Explains an auth failure when the request was sent without a key.
fn missing_key_hint(status: StatusCode, config: &Config) -> Option<String> {
    let auth_failed = matches!(status, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN);
    (auth_failed && config.api_key.is_none()).then(|| {
        format!(
            "\nNo API key found. Set SHELLY_API_KEY or {}, or add your API key to the config file.",
            config.kind.provider_env_var()
        )
    })
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
    fn hints_at_missing_key_only_on_auth_failure_without_key() {
        let hint = |status, key: Option<&str>| {
            let config = Config {
                kind: ProviderKind::OpenAi,
                api_key: key.map(str::to_string),
                base_url: String::new(),
                model: String::new(),
            };
            missing_key_hint(status, &config)
        };

        for status in [StatusCode::UNAUTHORIZED, StatusCode::FORBIDDEN] {
            assert!(hint(status, None).unwrap().contains("OPENAI_API_KEY"));
            assert_eq!(hint(status, Some("k")), None);
        }
        assert_eq!(hint(StatusCode::NOT_FOUND, None), None);
    }

    #[test]
    fn auth_header_matches_provider_and_is_omitted_without_key() {
        let auth_headers = |kind, key: Option<&str>| {
            let config = Config {
                kind,
                api_key: key.map(str::to_string),
                base_url: String::new(),
                model: String::new(),
            };
            let request = with_auth(Client::new().post("http://localhost/"), &config)
                .build()
                .unwrap();
            ["authorization", "x-api-key"].map(|name| {
                request
                    .headers()
                    .get(name)
                    .map(|value| value.to_str().unwrap().to_string())
            })
        };

        for kind in [ProviderKind::OpenAi, ProviderKind::Anthropic] {
            assert_eq!(auth_headers(kind, None), [None, None]);
        }
        assert_eq!(
            auth_headers(ProviderKind::OpenAi, Some("k")),
            [Some("Bearer k".to_string()), None]
        );
        assert_eq!(
            auth_headers(ProviderKind::Anthropic, Some("k")),
            [None, Some("k".to_string())]
        );
    }

    #[test]
    fn only_safe_label_skips_confirmation() {
        let with_risk = |risk: &str| ShellCommand {
            command: "true".to_string(),
            explanation: String::new(),
            risk: risk.to_string(),
        };

        for risk in ["safe", "Safe", "SAFE"] {
            assert!(!with_risk(risk).is_risky(), "{risk:?} should be safe");
        }
        for risk in [
            "risky",
            "RISKY",
            "destructive",
            "high",
            "unsafe",
            "unknown",
            "",
        ] {
            assert!(with_risk(risk).is_risky(), "{risk:?} should be risky");
        }
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
