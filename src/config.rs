use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize, Default)]
struct ConfigFile {
    openai: Option<OpenAiConfig>,
}

#[derive(Deserialize, Default)]
struct OpenAiConfig {
    api_key: Option<String>,
    model: Option<String>,
}

pub struct Config {
    pub api_key: String,
    pub model: String,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path()?;
        let env_key = std::env::var("SHELLY_API_KEY")
            .or_else(|_| std::env::var("OPENAI_API_KEY"))
            .ok()
            .filter(|k| !k.is_empty());

        if !path.exists() {
            match env_key {
                Some(key) => {
                    return Ok(Config {
                        api_key: key,
                        model: "gpt-4o-mini".to_string(),
                    });
                }
                None => {
                    Self::create_default(&path)?;
                    return Err(format!(
                        "Config file created at {}\nSet SHELLY_API_KEY (or OPENAI_API_KEY), or add your OpenAI API key to the config and try again.",
                        path.display()
                    ));
                }
            }
        }

        let contents =
            fs::read_to_string(&path).map_err(|e| format!("Failed to read config file: {e}"))?;

        let config: ConfigFile =
            toml::from_str(&contents).map_err(|e| format!("Failed to parse config file: {e}"))?;

        let openai = config.openai.unwrap_or_default();
        let api_key = env_key.or(openai.api_key).unwrap_or_default();

        if api_key.is_empty() || api_key == "YOUR_API_KEY_HERE" {
            return Err(format!(
                "No API key found. Set SHELLY_API_KEY (or OPENAI_API_KEY), or add your OpenAI API key to {}",
                path.display()
            ));
        }

        Ok(Config {
            api_key,
            model: openai.model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
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

        let default_config = r#"[openai]
api_key = "YOUR_API_KEY_HERE"
# model = "gpt-4o-mini"
"#;

        fs::write(path, default_config)
            .map_err(|e| format!("Failed to write default config: {e}"))?;

        Ok(())
    }
}
