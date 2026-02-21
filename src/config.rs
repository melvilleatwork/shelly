use std::fs;
use std::path::PathBuf;

use serde::Deserialize;

#[derive(Deserialize)]
struct ConfigFile {
    openai: OpenAiConfig,
}

#[derive(Deserialize)]
struct OpenAiConfig {
    api_key: String,
    model: Option<String>,
}

pub struct Config {
    pub api_key: String,
    pub model: String,
}

impl Config {
    pub fn load() -> Result<Self, String> {
        let path = Self::config_path()?;

        if !path.exists() {
            Self::create_default(&path)?;
            return Err(format!(
                "Config file created at {}\nPlease add your OpenAI API key and try again.",
                path.display()
            ));
        }

        let contents = fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read config file: {e}"))?;

        let config: ConfigFile = toml::from_str(&contents)
            .map_err(|e| format!("Failed to parse config file: {e}"))?;

        if config.openai.api_key.is_empty() || config.openai.api_key == "YOUR_API_KEY_HERE" {
            return Err(format!(
                "Please set your OpenAI API key in {}",
                path.display()
            ));
        }

        Ok(Config {
            api_key: config.openai.api_key,
            model: config.openai.model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
        })
    }

    fn config_path() -> Result<PathBuf, String> {
        let config_dir = dirs::config_dir()
            .ok_or("Could not determine config directory")?;
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
