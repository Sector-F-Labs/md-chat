use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use toml;
use dirs;

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct AppConfig {
    pub openai_api_key: Option<String>,
    pub api_url: String,
    pub models: Option<Vec<String>>,
}

pub fn get_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("MD-Chat").join("config.toml"))
}

pub fn get_completions_url(base_url: &str) -> String {
    format!("{}/v1/chat/completions", base_url)
}

pub fn load_or_create_config() -> AppConfig {
    let default_config = AppConfig {
        openai_api_key: None,
        api_url: "https://api.openai.com".to_string(),
        models: None,
    };
    if let Some(path) = get_config_path() {
        if !path.exists() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let toml_str = toml::to_string_pretty(&default_config).unwrap();
            let _ = fs::write(&path, toml_str);
            return default_config;
        }
        if let Ok(contents) = fs::read_to_string(&path) {
            toml::from_str(&contents).unwrap_or(default_config)
        } else {
            default_config
        }
    } else {
        default_config
    }
}

pub fn default_models() -> Vec<String> {
    vec![
        "gemini-2.0-flash".to_string(),
        "gpt-4.1".to_string(),
        "gpt-4o-mini".to_string(),
        "gpt-4o".to_string(),
    ]
}
