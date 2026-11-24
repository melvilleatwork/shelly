use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

const SHELLY_KEY_VAR: &str = "SHELLY_API_KEY";
const PLACEHOLDER_KEY: &str = "YOUR_API_KEY_HERE";

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

    pub fn provider_env_var(self) -> &'static str {
        match self {
            Self::OpenAi => "OPENAI_API_KEY",
            Self::Anthropic => "ANTHROPIC_API_KEY",
        }
    }
}

pub struct Config {
    pub kind: ProviderKind,
    /// `None` only for a custom `base_url`, where the endpoint may not need a key (e.g. Ollama).
    pub api_key: Option<String>,
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

        Self::resolve(file, &|name| std::env::var(name).ok(), &path)
    }

    /// Pure resolution step, with the environment injected so it can be tested.
    fn resolve(
        file: ProviderConfig,
        env: &dyn Fn(&str) -> Option<String>,
        path: &Path,
    ) -> Result<Self, String> {
        let env_key = |name: &str| usable_key(env(name));
        let provider = file.provider.unwrap_or_default();
        let base_url = file.base_url.unwrap_or_default();
        let shelly_key = env_key(SHELLY_KEY_VAR);
        let file_key = usable_key(file.api_key);

        // Provider kind: explicit setting > inference from base_url > ANTHROPIC_API_KEY being
        // the only key available > openai
        let unspecified = provider.is_empty() && base_url.is_empty();
        let only_anthropic_key = shelly_key.is_none()
            && file_key.is_none()
            && env_key(ProviderKind::OpenAi.provider_env_var()).is_none()
            && env_key(ProviderKind::Anthropic.provider_env_var()).is_some();
        let kind = if unspecified && only_anthropic_key {
            ProviderKind::Anthropic
        } else {
            ProviderKind::detect(&provider, &base_url)?
        };

        // Key resolution: SHELLY_API_KEY > provider-specific env var > config file
        let api_key = shelly_key
            .or_else(|| env_key(kind.provider_env_var()))
            .or(file_key);

        if api_key.is_none() && base_url.is_empty() {
            let vars = if unspecified {
                format!(
                    "{SHELLY_KEY_VAR}, {} or {}",
                    ProviderKind::OpenAi.provider_env_var(),
                    ProviderKind::Anthropic.provider_env_var()
                )
            } else {
                format!("{SHELLY_KEY_VAR} or {}", kind.provider_env_var())
            };
            return Err(format!(
                "No API key found. Set {vars}, or add your API key to {}",
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
# provider = "openai"              # "openai" or "anthropic"; inferred when omitted
api_key = "YOUR_API_KEY_HERE"      # or set SHELLY_API_KEY / OPENAI_API_KEY / ANTHROPIC_API_KEY
# base_url = "https://api.openai.com/v1"   # any OpenAI-compatible endpoint; key optional when set
# model = "gpt-4o-mini"
"#;

        fs::write(path, default_config)
            .map_err(|e| format!("Failed to write default config: {e}"))?;

        Ok(())
    }
}

/// Treats blank values and the default-config placeholder as "not set".
fn usable_key(key: Option<String>) -> Option<String> {
    key.filter(|k| !k.trim().is_empty() && k != PLACEHOLDER_KEY)
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
        assert_ne!(ProviderKind::OpenAi.default_model(), "");
        assert_ne!(ProviderKind::Anthropic.default_model(), "");
    }

    fn resolve(file: &str, env: &[(&str, &str)]) -> Result<Config, String> {
        let file: ConfigFile =
            toml::from_str(&format!("[provider]\n{file}")).map_err(|e| e.to_string())?;
        let lookup = |name: &str| {
            env.iter()
                .find(|(key, _)| *key == name)
                .map(|(_, value)| (*value).to_string())
        };
        Config::resolve(
            file.provider.unwrap_or_default(),
            &lookup,
            Path::new("config.toml"),
        )
    }

    #[test]
    fn lone_anthropic_key_selects_anthropic() {
        for file in ["", "api_key = \"YOUR_API_KEY_HERE\"", "api_key = \"\""] {
            let env = [("ANTHROPIC_API_KEY", "ant"), ("OPENAI_API_KEY", "")];
            let config = resolve(file, &env).unwrap();
            assert_eq!(config.kind, ProviderKind::Anthropic);
            assert_eq!(config.api_key.as_deref(), Some("ant"));
            assert_eq!(config.base_url, "https://api.anthropic.com/v1");
            assert_eq!(config.model, ProviderKind::Anthropic.default_model());
        }
    }

    #[test]
    fn anthropic_key_does_not_override_other_choices() {
        let ant = ("ANTHROPIC_API_KEY", "ant");
        let cases = [
            ("api_key = \"file\"", vec![ant], Some("file")),
            ("", vec![ant, ("OPENAI_API_KEY", "oai")], Some("oai")),
            ("", vec![ant, ("SHELLY_API_KEY", "sh")], Some("sh")),
            ("base_url = \"http://localhost:11434/v1\"", vec![ant], None),
        ];
        for (file, env, key) in cases {
            let config = resolve(file, &env).unwrap();
            assert_eq!(config.kind, ProviderKind::OpenAi, "{file} {env:?}");
            assert_eq!(config.api_key.as_deref(), key, "{file} {env:?}");
        }
        assert!(resolve("provider = \"openai\"", &[ant]).is_err());
    }

    #[test]
    fn empty_keys_fall_through_to_next_source() {
        let key = |file: &str, env: &[(&str, &str)]| resolve(file, env).unwrap().api_key;

        let blank_shelly = [("SHELLY_API_KEY", ""), ("OPENAI_API_KEY", "oai")];
        assert_eq!(key("", &blank_shelly).as_deref(), Some("oai"));

        let all_blank = [("SHELLY_API_KEY", " "), ("OPENAI_API_KEY", "")];
        let file = "api_key = \"file\"";
        assert_eq!(key(file, &all_blank).as_deref(), Some("file"));
    }

    #[test]
    fn key_precedence_is_shelly_then_provider_then_file() {
        let file = "provider = \"anthropic\"\napi_key = \"file\"";
        let key = |env: &[(&str, &str)]| resolve(file, env).unwrap().api_key;

        let both = [("SHELLY_API_KEY", "sh"), ("ANTHROPIC_API_KEY", "ant")];
        assert_eq!(key(&both).as_deref(), Some("sh"));
        assert_eq!(key(&[("ANTHROPIC_API_KEY", "ant")]).as_deref(), Some("ant"));
        assert_eq!(key(&[("OPENAI_API_KEY", "oai")]).as_deref(), Some("file"));
    }

    #[test]
    fn key_is_optional_only_with_custom_base_url() {
        let file = "base_url = \"http://localhost:11434/v1\"\napi_key = \"YOUR_API_KEY_HERE\"";
        assert_eq!(resolve(file, &[]).unwrap().api_key, None);

        let error = resolve("", &[]).err().unwrap();
        for expected in [
            "SHELLY_API_KEY",
            "OPENAI_API_KEY",
            "ANTHROPIC_API_KEY",
            "config.toml",
        ] {
            assert!(error.contains(expected), "{error}");
        }
        assert!(resolve("provider = \"anthropic\"", &[]).is_err());
    }
}
