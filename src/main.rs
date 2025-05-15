use app::{ChatMessage, MyApp};
use dirs;
use eframe::egui::{IconData, ViewportBuilder};
use image;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

const APP_NAME: &str = "MD-Chat";

mod app;
mod openai;

#[derive(Serialize, Deserialize, Debug, Default)]
struct AppConfig {
    openai_api_key: Option<String>,
    api_url: String,
    models: Option<Vec<String>>,
}

fn get_config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|dir| dir.join("MD-Chat").join("config.toml"))
}

fn get_completions_url(base_url: &str) -> String {
    format!("{}/v1/chat/completions", base_url)
}

fn load_or_create_config() -> AppConfig {
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

fn default_models() -> Vec<String> {
    vec![
        "gemini-2.0-flash".to_string(),
        "gpt-4.1".to_string(),
        "gpt-4o-mini".to_string(),
        "gpt-4o".to_string(),
    ]
}

async fn fetch_history() -> Result<Vec<ChatMessage>, String> {
    let url = format!("http://localhost:3017/partition/default/instance/default/command/view/15",);
    let client = reqwest::Client::new();
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    let text = response.text().await.map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

fn main() -> eframe::Result<()> {
    // Load the icon image
    let icon_bytes = include_bytes!("../assets/icon.iconset/icon_256x256.png");
    let image = image::load_from_memory(icon_bytes)
        .expect("Failed to load icon")
        .into_rgba8();
    let (width, height) = image.dimensions();
    let icon_rgba = image.into_raw();
    let icon_data = IconData {
        rgba: icon_rgba,
        width,
        height,
    };

    // Set up native options with the icon
    let native_options = eframe::NativeOptions {
        persist_window: true,
        viewport: ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_position([50.0, 50.0])
            .with_app_id(APP_NAME)
            .with_min_inner_size([400.0, 300.0])
            .with_resizable(true)
            .with_icon(Arc::new(icon_data)),
        centered: true,
        ..Default::default()
    };

    // Run the application
    eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
}
