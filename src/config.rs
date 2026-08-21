use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Default)]
struct ConfigFile {
    provider: Option<ProviderConfig>,
}

#[derive(Deserialize, Default)]
struct ProviderConfig {
    provider: Option<String>,
    api_key: Option<String>,
    base_url: Option<String>,
    model: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum ProviderKind {
    OpenAi,
    Anthropic,
}

impl ProviderKind {
    pub fn detect(name: &str, base_url: &str) -> Result<Self, String> {
        match name.to_lowercase().as_str() {
            "" => {
                if base_url.contains("anthropic") {
                    Ok(Self::Anthropic)
                } else {
                    Ok(Self::OpenAi)
                }
            }
            "openai" | "openai-compatible" => Ok(Self::OpenAi),
            "anthropic" => Ok(Self::Anthropic),
            other => Err(format!(
                "Unknown provider \"{other}\". Supported: \"openai\", \"anthropic\" (any OpenAI-compatible endpoint works with \"openai\")."
            )),
        }
    }

    fn default_base_url(self) -> &'static str {
        match self {
            Self::OpenAi => "https://api.openai.com/v1",
            Self::Anthropic => "https://api.anthropic.com/v1",
        }
    }

    fn default_model(self) -> &'static str {
        match self {
            Self::OpenAi => "gpt-4o-mini",
            Self::Anthropic => "claude-sonnet-4-5",
        }
    }

    fn provider_env_var(self) -> &'static str {
        match self {
            Self::OpenAi => "OPENAI_API_KEY",
            Self::Anthropic => "ANTHROPIC_API_KEY",
        }
    }
}

pub struct Config {
    pub kind: ProviderKind,
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path()?;

        let file = if path.exists() {
            let contents = fs::read_to_string(&path)
                .map_err(|e| format!("Failed to read config file: {e}"))?;
            let file: ConfigFile = toml::from_str(&contents)
                .map_err(|e| format!("Failed to parse config file: {e}"))?;
            file.provider.unwrap_or_default()
        } else {
            Self::create_default(&path)?;
            ProviderConfig::default()
        };

        // Resolve provider kind: explicit setting > inference from base_url > openai
        let base_url = file.base_url.clone().unwrap_or_default();
        let kind = ProviderKind::detect(file.provider.as_deref().unwrap_or(""), &base_url)?;

        // Key resolution: SHELLY_API_KEY > provider-specific env var > config file
        let api_key = std::env::var("SHELLY_API_KEY")
            .or_else(|_| std::env::var(kind.provider_env_var()))
            .ok()
            .filter(|k| !k.is_empty())
            .or(file.api_key)
            .unwrap_or_default();

        if api_key.is_empty() || api_key == "YOUR_API_KEY_HERE" {
            return Err(format!(
                "No API key found. Set SHELLY_API_KEY or {}, or add your API key to {}",
                kind.provider_env_var(),
                path.display()
            ));
        }

        Ok(Config {
            kind,
            api_key,
            model: file
                .model
                .unwrap_or_else(|| kind.default_model().to_string()),
            base_url: if base_url.is_empty() {
                kind.default_base_url().to_string()
            } else {
                base_url
            },
        })
    }

    fn config_path() -> Result<PathBuf, String> {
        let config_dir = dirs::config_dir().ok_or("Could not determine config directory")?;
        Ok(config_dir.join("shelly").join("config.toml"))
    }

    fn create_default(path: &PathBuf) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create config directory: {e}"))?;
        }

        let default_config = r#"[provider]
# provider = "openai"              # "openai" or "anthropic"; inferred from base_url when omitted
api_key = "YOUR_API_KEY_HERE"      # or set SHELLY_API_KEY / OPENAI_API_KEY / ANTHROPIC_API_KEY
# base_url = "https://api.openai.com/v1"   # any OpenAI-compatible endpoint works
# model = "gpt-4o-mini"
"#;

        fs::write(path, default_config)
            .map_err(|e| format!("Failed to write default config: {e}"))?;

        Ok(())
    }
}

#[cfg(test)]
#[expect(clippy::unwrap_used, reason = "test code")]
mod tests {
    use super::*;

    #[test]
    fn detects_explicit_providers() {
        assert_eq!(
            ProviderKind::detect("openai", "").unwrap(),
            ProviderKind::OpenAi
        );
        assert_eq!(
            ProviderKind::detect("anthropic", "").unwrap(),
            ProviderKind::Anthropic
        );
    }

    #[test]
    fn infers_provider_from_base_url() {
        assert_eq!(
            ProviderKind::detect("", "https://api.anthropic.com/v1").unwrap(),
            ProviderKind::Anthropic
        );
        assert_eq!(
            ProviderKind::detect("", "http://localhost:11434/v1").unwrap(),
            ProviderKind::OpenAi
        );
    }

    #[test]
    fn rejects_unknown_provider() {
        assert!(ProviderKind::detect("wat", "").is_err());
    }

    #[test]
    fn defaults_are_sane() {
        assert_eq!(
            ProviderKind::OpenAi.default_base_url(),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            ProviderKind::Anthropic.default_base_url(),
            "https://api.anthropic.com/v1"
        );
        assert!(!ProviderKind::OpenAi.default_model().is_empty());
        assert!(!ProviderKind::Anthropic.default_model().is_empty());
    }
}
