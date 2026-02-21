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

    // Strip markdown fences if present
    let clean = content
        .trim()
        .strip_prefix("```json")
        .or_else(|| content.trim().strip_prefix("```"))
        .unwrap_or(content.trim());
    let clean = clean.strip_suffix("```").unwrap_or(clean).trim();

    let shell_cmd: ShellCommand = serde_json::from_str(clean)
        .map_err(|e| format!("Failed to parse LLM response as JSON: {e}\nRaw response: {content}"))?;

    Ok(shell_cmd)
}
