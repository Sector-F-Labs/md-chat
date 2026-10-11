//! MD-Chat: a desktop chat client for OpenAI-compatible APIs, with markdown
//! replies. Built on egui and Patina.

use std::sync::Arc;

mod app;
mod config;
mod export;
mod history;
mod openai;
mod screenshot;

const APP_ID: &str = "com.sectorflabs.MdChat";

fn main() -> eframe::Result {
    let config = config::AppConfig::load_or_create();
    let icon = image::load_from_memory(include_bytes!("../assets/icon.iconset/icon_256x256.png"))
        .expect("the embedded icon")
        .into_rgba8();
    let (width, height) = icon.dimensions();
    let options = eframe::NativeOptions {
        viewport: patina::frame::viewport(
            egui::ViewportBuilder::default()
                .with_title("MD-Chat")
                .with_app_id(APP_ID)
                .with_inner_size([800.0, 600.0])
                .with_min_inner_size([400.0, 300.0])
                .with_icon(Arc::new(egui::IconData { rgba: icon.into_raw(), width, height })),
            !config.system_title_bar,
        ),
        ..Default::default()
    };
    eframe::run_native("MD-Chat", options, Box::new(|cc| Ok(Box::new(app::App::new(cc, config)))))
}
