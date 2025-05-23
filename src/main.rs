use app::{ChatMessage, MyApp};
use dirs;
use eframe::egui::{IconData, ViewportBuilder};
use image;
use serde_json;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

const APP_NAME: &str = "MD-Chat";

mod app;
mod openai;
mod config;

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
