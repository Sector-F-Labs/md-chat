//! `~/.config/MD-Chat/config.toml`: the API to talk to, and the window settings.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// A local [Ollama](https://ollama.com), the default API.
pub const OLLAMA: &str = "http://localhost:11434";

/// What earlier versions wrote as the default, before anyone chose it.
const OLD_DEFAULT_URL: &str = "https://api.openai.com";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppConfig {
    pub openai_api_key: Option<String>,
    /// The API's base URL; requests go to `<api_url>/v1/chat/completions`.
    pub api_url: String,
    /// The last model picked, chosen again if the API still has it.
    #[serde(default)]
    pub model: Option<String>,
    /// The chosen skin's name; the app's `HOME_SKIN` when unset.
    #[serde(default)]
    pub skin: Option<String>,
    /// The desktop's own title bar instead of Patina's.
    #[serde(default)]
    pub system_title_bar: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            openai_api_key: None,
            api_url: OLLAMA.to_string(),
            model: None,
            skin: None,
            system_title_bar: false,
        }
    }
}

impl AppConfig {
    /// The config, written with defaults on first run.
    pub fn load_or_create() -> Self {
        let Some(path) = path() else { return Self::default() };
        if !path.exists() {
            let config = Self::default();
            config.save();
            return config;
        }
        let config: Self = fs::read_to_string(&path).ok().and_then(|s| toml::from_str(&s).ok()).unwrap_or_default();
        config.migrated()
    }

    /// OpenAI's URL without a key was only ever the old default, which can't
    /// work: point it at Ollama instead.
    fn migrated(mut self) -> Self {
        let no_key = self.openai_api_key.as_deref().is_none_or(|k| k.trim().is_empty());
        if self.api_url.trim_end_matches('/') == OLD_DEFAULT_URL && no_key {
            self.api_url = OLLAMA.to_string();
        }
        self
    }

    pub fn save(&self) {
        let Some(path) = path() else { return };
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(text) = toml::to_string_pretty(self) {
            let _ = fs::write(&path, text);
        }
    }

    pub fn completions_url(&self) -> String {
        format!("{}/v1/chat/completions", self.api_url.trim_end_matches('/'))
    }

    /// The API's model list (OpenAI's `GET /v1/models`, which Ollama has too).
    pub fn models_url(&self) -> String {
        format!("{}/v1/models", self.api_url.trim_end_matches('/'))
    }
}

fn path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("MD-Chat").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_old_config_without_window_settings_still_loads() {
        let config: AppConfig = toml::from_str("api_url = \"http://localhost:3017\"\n").unwrap();
        assert_eq!(config.completions_url(), "http://localhost:3017/v1/chat/completions");
        assert_eq!(config.skin, None);
        assert!(!config.system_title_bar);
    }

    #[test]
    fn the_old_default_without_a_key_becomes_ollama() {
        let old: AppConfig = toml::from_str("api_url = \"https://api.openai.com\"\n").unwrap();
        assert_eq!(old.migrated().api_url, OLLAMA);
        let chosen: AppConfig =
            toml::from_str("api_url = \"https://api.openai.com\"\nopenai_api_key = \"sk-test\"\n").unwrap();
        assert_eq!(chosen.migrated().api_url, "https://api.openai.com");
    }

    #[test]
    fn a_trailing_slash_in_the_url_is_fine() {
        let config = AppConfig { api_url: "https://api.openai.com/".into(), ..Default::default() };
        assert_eq!(config.completions_url(), "https://api.openai.com/v1/chat/completions");
    }
}
