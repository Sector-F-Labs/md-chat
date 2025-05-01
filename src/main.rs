// main.rs
use eframe::egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};

struct MyApp {
    markdown_text: String,
    cache: CommonMarkCache,
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            // Make the area scrollable in case content is long
            egui::ScrollArea::vertical().show(ui, |ui| {
                // Render the markdown content to the UI
                CommonMarkViewer::new().show(ui, &mut self.cache, &self.markdown_text);
            });
        });
    }
}

fn main() {
    // Read the markdown file (change "example.md" to your file path)
    let md_path = "example.md";
    let markdown_text = std::fs::read_to_string(md_path)
        .unwrap_or_else(|e| format!("**Error:** Could not read file `{}`.\n{}", md_path, e));

    // Create the app and run it
    let app = MyApp {
        markdown_text,
        cache: CommonMarkCache::default(),
    };
    let native_opts = eframe::NativeOptions::default();
    eframe::run_native(
        "Markdown Viewer",
        native_opts,
        Box::new(|_cc| Ok(Box::new(app))),
    )
    .expect("Failed to launch eframe application");
}
