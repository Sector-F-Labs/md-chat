// main.rs
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use serde_json;

struct MyApp {
    markdown_text: String,
    cache: CommonMarkCache,
    window_info: Option<egui::ViewportInfo>,
    dark_mode: bool,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>, markdown_text: String) -> Self {
        // Try to load saved window info
        let window_info = if let Some(storage) = cc.storage {
            if let Some(json) = storage.get_string("window_info") {
                serde_json::from_str(&json).ok()
            } else {
                None
            }
        } else {
            None
        };

        // Try to load theme preference, default to dark mode
        let dark_mode = if let Some(storage) = cc.storage {
            storage.get_string("dark_mode")
                .and_then(|s| s.parse().ok())
                .unwrap_or(true)
        } else {
            true
        };

        // Set the initial theme
        cc.egui_ctx.set_visuals(if dark_mode {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        });

        Self {
            markdown_text,
            cache: CommonMarkCache::default(),
            window_info,
            dark_mode,
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Store current window info
        self.window_info = Some(ctx.input(|i| i.viewport().clone()));
        
        // Add top panel with theme toggle
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let theme_btn = ui.button(if self.dark_mode { "☀ Light" } else { "🌙 Dark" });
                    if theme_btn.clicked() {
                        self.dark_mode = !self.dark_mode;
                        ctx.set_visuals(if self.dark_mode {
                            egui::Visuals::dark()
                        } else {
                            egui::Visuals::light()
                        });
                    }
                });
            });
            ui.add_space(4.0);
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            // Make the area scrollable in case content is long
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Render the markdown content to the UI
                CommonMarkViewer::new().show(ui, &mut self.cache, &self.markdown_text);
            });
        });
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Save window info
        if let Some(window_info) = &self.window_info {
            if let Ok(json) = serde_json::to_string(window_info) {
                storage.set_string("window_info", json);
            }
        }
        
        // Save theme preference
        storage.set_string("dark_mode", self.dark_mode.to_string());
    }

    fn persist_egui_memory(&self) -> bool {
        true
    }
}

fn main() {
    // Read the markdown file (change "example.md" to your file path)
    let md_path = "example.md";
    let markdown_text = std::fs::read_to_string(md_path)
        .unwrap_or_else(|e| format!("**Error:** Could not read file `{}`.\n{}", md_path, e));

    let native_options = eframe::NativeOptions {
        persist_window: true,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_position([50.0, 50.0]) // Set initial position if no saved state
            .with_app_id("md-chat") // Unique app ID for persistence
            .with_min_inner_size([400.0, 300.0]) // Prevent window from becoming too small
            .with_resizable(true), // Allow window resizing
        centered: true, // Center on first launch if no position is saved
        ..Default::default()
    };

    eframe::run_native(
        "Markdown Viewer",
        native_options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc, markdown_text)))),
    )
    .expect("Failed to launch eframe application");
}
