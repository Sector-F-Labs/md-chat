// main.rs
use dirs;
use eframe::egui;
use eframe::egui::{IconData, ViewportBuilder, FontDefinitions, FontFamily};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use image;
use serde::{Deserialize, Serialize};
use serde_json;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};
use whoami;

const APP_NAME: &str = "MD-Chat";

// Add your preferred models here
const AVAILABLE_MODELS: &[&str] = &["gemini-2.0-flash", "gpt-4.1", "gpt-4o-mini", "gpt-4o"];

mod openai;
use openai::Role;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum EditMode {
    Normal,
    Insert,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct ChatMessage {
    role: Role,
    content: String,
}

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

#[allow(dead_code)]
struct MyApp {
    dark_mode: bool,
    messages: Vec<ChatMessage>,
    input: String,
    http_client: reqwest::Client,
    response_rx: Receiver<Result<String, String>>,
    request_tx: Sender<String>,
    is_processing: bool,
    markdown_cache: CommonMarkCache,
    selected_model: String,
    history_rx: Option<Receiver<Result<Vec<ChatMessage>, String>>>,
    models: Vec<String>,
    edit_mode: EditMode,
    scroll_offset: f32,
    pending_scroll: Option<f32>,
    current_scroll_offset: f32,
    message_tops: Vec<f32>,
    copy_button_tops: Vec<f32>,
    last_scroll_area_height: f32,
}

async fn fetch_history() -> Result<Vec<ChatMessage>, String> {
    let url = format!("http://localhost:3017/partition/default/instance/default/command/view/15",);
    let client = reqwest::Client::new();
    let response = client.get(url).send().await.map_err(|e| e.to_string())?;
    let text = response.text().await.map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

#[allow(dead_code)]
impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load or create config
        let config = load_or_create_config();
        // Initialize HTTP client
        let http_client = reqwest::Client::new();

        // Set up channels for async communication
        let (request_tx, request_rx): (Sender<String>, Receiver<String>) = channel();
        let (response_tx, response_rx): (
            Sender<Result<String, String>>,
            Receiver<Result<String, String>>,
        ) = channel();

        // Spawn background thread for handling API requests
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            while let Ok(message) = request_rx.recv() {
                let tx = response_tx.clone();
                let (content, model) = message.split_once('\0').unwrap();
                let api_key = config.openai_api_key.as_deref().unwrap_or("");
                let api_url = get_completions_url(&config.api_url);
                rt.block_on(async {
                    let result =
                        openai::send_openai_request(content, model, api_key, api_url.as_str())
                            .await;
                    tx.send(result).unwrap();
                });
            }
        });

        // Fetch history synchronously
        let mut messages = Vec::new();
        // Add initial system message
        messages.push(ChatMessage {
            role: Role::System,
            content:
                "You are a helpful assistant. You can use markdown formatting in your responses."
                    .to_string(),
        });
        // Fetch history and append
        let mut pending_scroll = None;
        if let Ok(rt) = tokio::runtime::Runtime::new() {
            if let Ok(history) = rt.block_on(fetch_history()) {
                messages.extend(history);
                pending_scroll = Some(100_000.0);
            }
        }
        let models = config.models.clone().unwrap_or_else(default_models);

        // Set up custom font: Lexend
        let mut fonts = FontDefinitions::default();
        // Load the font bytes at compile time
        let lexend_bytes = include_bytes!("../assets/Lexend-VariableFont_wght.ttf").to_vec();
        fonts.font_data.insert(
            "lexend".to_owned(),
            egui::FontData::from_owned(lexend_bytes).into(),
        );
        // Load the emoji font bytes at compile time (monochrome version)
        let emoji_bytes = include_bytes!("../assets/NotoEmoji-VariableFont_wght.ttf").to_vec();
        fonts.font_data.insert(
            "emoji".to_owned(),
            egui::FontData::from_owned(emoji_bytes).into(),
        );
        // Use Lexend for both proportional and monospace, with emoji as fallback
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().insert(0, "lexend".to_owned());
        fonts.families.get_mut(&FontFamily::Proportional).unwrap().push("emoji".to_owned());
        fonts.families.get_mut(&FontFamily::Monospace).unwrap().insert(0, "lexend".to_owned());
        fonts.families.get_mut(&FontFamily::Monospace).unwrap().push("emoji".to_owned());
        cc.egui_ctx.set_fonts(fonts);

        Self {
            dark_mode: true,
            messages,
            input: String::new(),
            http_client,
            response_rx,
            request_tx,
            is_processing: false,
            markdown_cache: CommonMarkCache::default(),
            selected_model: models.get(0).cloned().unwrap_or_default(),
            history_rx: None,
            models,
            edit_mode: EditMode::Insert,
            scroll_offset: 0.0,
            pending_scroll,
            current_scroll_offset: 0.0,
            message_tops: Vec::new(),
            copy_button_tops: Vec::new(),
            last_scroll_area_height: 0.0,
        }
    }

    fn send_message(&mut self) {
        if self.input.trim().is_empty() || self.is_processing {
            return;
        }

        let message = ChatMessage {
            role: Role::User,
            content: self.input.clone(),
        };
        self.messages.push(message);

        // Send request
        self.request_tx.send(self.input.clone()).ok();
        self.input.clear();
        self.is_processing = true;
    }

    fn refresh_history(&mut self) {
        if self.is_processing {
            return;
        }
        self.is_processing = true;
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            let result = rt.block_on(fetch_history());
            tx.send(result).unwrap();
        });
        self.history_rx = Some(rx);
    }

    fn handle_insert_mode(&mut self, text_edit: &egui::Response) {
        text_edit.request_focus();
    }

    fn handle_normal_mode(&mut self, text_edit: &egui::Response, ctx: &egui::Context) {
        if text_edit.has_focus() {
            // Remove focus from the input box
            ctx.memory_mut(|mem| mem.surrender_focus(text_edit.id));
            // If the user just clicked, switch to Insert mode
            self.edit_mode = EditMode::Insert;
        }
    }
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Top panel for theme toggle
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button(if self.dark_mode { "🌙" } else { "☀" }).clicked() {
                    self.dark_mode = !self.dark_mode;
                    if self.dark_mode {
                        ctx.set_visuals(egui::Visuals::dark());
                    } else {
                        ctx.set_visuals(egui::Visuals::light());
                    }
                }
                ui.separator();
                if ui
                    .button("🔄 Refresh")
                    .on_hover_text("Fetch history")
                    .clicked()
                    && !self.is_processing
                {
                    self.refresh_history();
                }
                if self.is_processing {
                    ui.add(egui::Spinner::new());
                }
                egui::ComboBox::from_label("Model")
                    .selected_text(&self.selected_model)
                    .show_ui(ui, |ui| {
                        for model in &self.models {
                            ui.selectable_value(
                                &mut self.selected_model,
                                model.clone(),
                                model,
                            );
                        }
                    });
                // Add a spacer to push the mode indicator to the right
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let (mode_text, bg_color, fg_color) = match self.edit_mode {
                        EditMode::Normal => ("NORMAL", egui::Color32::BLACK, egui::Color32::WHITE),
                        EditMode::Insert => ("INSERT", egui::Color32::WHITE, egui::Color32::BLACK),
                    };
                    ui.label(
                        egui::RichText::new(mode_text)
                            .strong()
                            .background_color(bg_color)
                            .color(fg_color)
                            .monospace()
                            .size(16.0)
                    );
                });
            });
        });

        // Handle modal editing key events
        let input = ctx.input(|i| i.clone());
        match self.edit_mode {
            EditMode::Insert => {
                if input.key_pressed(egui::Key::Escape) {
                    self.edit_mode = EditMode::Normal;
                }
            }
            EditMode::Normal => {
                if input.key_pressed(egui::Key::I) && !input.modifiers.shift && !input.modifiers.ctrl && !input.modifiers.alt && !input.modifiers.mac_cmd && !input.modifiers.command {
                    self.edit_mode = EditMode::Insert;
                }
                // j/k scrolling
                let scroll_amount = 60.0; // One message height
                if input.key_pressed(egui::Key::R) && !input.modifiers.any() {
                    self.refresh_history();
                }
                if input.key_pressed(egui::Key::J) && !input.modifiers.shift {
                    let new_offset = self.current_scroll_offset + scroll_amount;
                    self.pending_scroll = Some(new_offset);
                }
                if input.key_pressed(egui::Key::K) && !input.modifiers.shift {
                    let new_offset = (self.current_scroll_offset - scroll_amount).max(0.0);
                    self.pending_scroll = Some(new_offset);
                }
                // Shift+J/K: scroll by one window height
                let window_height = self.last_scroll_area_height;
                if input.key_pressed(egui::Key::J) && input.modifiers.shift {
                    let new_offset = self.current_scroll_offset + window_height;
                    self.pending_scroll = Some(new_offset);
                }
                if input.key_pressed(egui::Key::K) && input.modifiers.shift {
                    let new_offset = (self.current_scroll_offset - window_height).max(0.0);
                    self.pending_scroll = Some(new_offset);
                }
                // G (Shift+g) to scroll to bottom, g to scroll to top
                if input.key_pressed(egui::Key::G) && input.modifiers.shift {
                    self.pending_scroll = Some(100_000.0); // Large value to ensure bottom
                }
                if input.key_pressed(egui::Key::G) && !input.modifiers.shift {
                    self.pending_scroll = Some(0.0);
                }
            }
        }

        // Bottom panel for input
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                // Make the text input take up as much width as possible
                let available_width = ui.available_width();
                let button_width = 80.0; // Reserve space for the button
                let text_edit = ui.add_sized(
                    [
                        available_width - button_width,
                        60.0, // or your preferred height
                    ],
                    egui::TextEdit::multiline(&mut self.input),
                );
                match self.edit_mode {
                    EditMode::Insert => {
                        self.handle_insert_mode(&text_edit);
                    }
                    EditMode::Normal => {
                        self.handle_normal_mode(&text_edit, ctx);
                    }
                }

                if self.is_processing {
                    ui.add(egui::Spinner::new());
                }

                let text_edit_height = 60.0;
                if self.edit_mode == EditMode::Insert && (ui.add(
                    egui::Button::new(if self.is_processing { "..." } else { "Send" })
                        .min_size(egui::vec2(button_width, text_edit_height))
                ).clicked()
                    || (ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift)))
                {
                    if !self.input.trim().is_empty() && !self.is_processing {
                        let message = std::mem::take(&mut self.input);
                        self.messages.push(ChatMessage {
                            role: Role::User,
                            content: message.clone(),
                        });
                        // Combine message and model with a null byte separator
                        let request = format!("{}\0{}", message, self.selected_model);
                        self.request_tx.send(request).unwrap();
                        self.is_processing = true;
                    }
                } else {
                    ui.add_enabled(false,
                        egui::Button::new(if self.is_processing { "..." } else { "Send" })
                            .min_size(egui::vec2(button_width, text_edit_height))
                    );
                }
            });
        });

        // Central panel for messages
        egui::CentralPanel::default().show(ctx, |ui| {
            let mut scroll_area = egui::ScrollArea::vertical();
            if let Some(offset) = self.pending_scroll {
                scroll_area = scroll_area.scroll_offset(egui::Vec2::new(0.0, offset));
            }
            // Track Copy button top positions
            self.copy_button_tops.clear();
            self.message_tops.clear();
            let mut y = 0.0;
            let output = scroll_area.show(ui, |ui| {
                // Store the visible height of the scroll area for window jumps
                self.last_scroll_area_height = ui.available_height();
                for message in &self.messages {
                    let before = ui.cursor().top();
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Track Copy button position
                            let copy_button_top = ui.cursor().top();
                            if ui
                                .button("Copy")
                                .on_hover_text("Copy entire message markdown")
                                .clicked()
                            {
                                ui.output_mut(|o| o.copied_text = message.content.clone());
                            }
                            self.copy_button_tops.push(copy_button_top);
                            // Add some spacing between the button and the text
                            ui.add_space(4.0);
                            let viewer = CommonMarkViewer::new();
                            viewer.show(ui, &mut self.markdown_cache, &message.content);
                        });
                    });
                    let after = ui.cursor().top();
                    self.message_tops.push(before);
                    y = after;
                    ui.add_space(8.0);
                }
            });
            // After rendering, update current_scroll_offset and clear pending_scroll
            self.current_scroll_offset = output.state.offset.y;
            self.pending_scroll = None;
        });

        // Check for history refresh result
        if let Some(rx) = &self.history_rx {
            if let Ok(result) = rx.try_recv() {
                match result {
                    Ok(history) => {
                        // Keep the system message, replace the rest
                        if !self.messages.is_empty() {
                            self.messages.truncate(1);
                        }
                        self.messages.extend(history);
                    }
                    Err(error) => {
                        self.messages.push(ChatMessage {
                            role: Role::System,
                            content: format!("Error fetching history: {}", error),
                        });
                    }
                }
                self.is_processing = false;
                self.history_rx = None;
            }
        }

        // Check for responses
        if let Ok(response) = self.response_rx.try_recv() {
            match response {
                Ok(content) => {
                    self.messages.push(ChatMessage {
                        role: Role::Assistant,
                        content,
                    });
                }
                Err(error) => {
                    self.messages.push(ChatMessage {
                        role: Role::System,
                        content: format!("Error: {}", error),
                    });
                }
            }
            self.is_processing = false;
        }
    }

    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        // Save theme preference
        storage.set_string("dark_mode", self.dark_mode.to_string());
    }

    fn persist_egui_memory(&self) -> bool {
        true
    }
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
