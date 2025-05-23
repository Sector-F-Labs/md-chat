use eframe::egui;
use eframe::egui::{FontDefinitions, FontFamily};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use serde::{Deserialize, Serialize};
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::openai::Role;
use crate::{fetch_history, openai};
use crate::config::{self, AppConfig};
use crate::components::top_panel::TopPanel;
use crate::components::input_panel::InputPanel;
use crate::components::message_list_panel::MessageListPanel;

mod components;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum EditMode {
    Normal,
    Insert,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

#[allow(dead_code)]
pub struct MyApp {
    pub dark_mode: bool,
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub http_client: reqwest::Client,
    pub response_rx: Receiver<Result<String, String>>,
    pub request_tx: Sender<String>,
    pub is_processing: bool,
    pub markdown_cache: CommonMarkCache,
    pub selected_model: String,
    pub history_rx: Option<Receiver<Result<Vec<ChatMessage>, String>>>,
    pub models: Vec<String>,
    edit_mode: EditMode,
    pub scroll_offset: f32,
    pub pending_scroll: Option<f32>,
    pub current_scroll_offset: f32,
    pub last_scroll_area_height: f32,
}

#[allow(dead_code)]
impl MyApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load or create config
        let app_config = config::load_or_create_config();
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
                let api_key = app_config.openai_api_key.as_deref().unwrap_or("");
                let api_url = config::get_completions_url(&app_config.api_url);
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
        let models = app_config.models.clone().unwrap_or_else(config::default_models);

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
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .insert(0, "lexend".to_owned());
        fonts
            .families
            .get_mut(&FontFamily::Proportional)
            .unwrap()
            .push("emoji".to_owned());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .insert(0, "lexend".to_owned());
        fonts
            .families
            .get_mut(&FontFamily::Monospace)
            .unwrap()
            .push("emoji".to_owned());
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
            last_scroll_area_height: 0.0,
        }
    }

    fn scroll_to_bottom(&mut self) {
        self.pending_scroll = Some(100_000.0);
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
        self.scroll_to_bottom();

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
}

impl eframe::App for MyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let top_panel_handler = TopPanel;
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            top_panel_handler.show(
                ctx,
                ui,
                &mut self.dark_mode,
                &self.is_processing,
                &mut self.selected_model,
                &self.models,
                &self.edit_mode,
                || self.refresh_history(),
            );
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
                if input.key_pressed(egui::Key::I)
                    && !input.modifiers.shift
                    && !input.modifiers.ctrl
                    && !input.modifiers.alt
                    && !input.modifiers.mac_cmd
                    && !input.modifiers.command
                {
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
                    self.scroll_to_bottom();
                }
                if input.key_pressed(egui::Key::G) && !input.modifiers.shift {
                    self.pending_scroll = Some(0.0);
                }
            }
        }

        // Bottom panel for input
        let input_panel_handler = InputPanel;
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            input_panel_handler.show(
                ctx,
                ui,
                &mut self.input,
                &self.is_processing,
                &mut self.edit_mode,
                |message_to_send: String| {
                    self.messages.push(ChatMessage {
                        role: Role::User,
                        content: message_to_send.clone(),
                    });
                    let request_content = format!("{} {}", message_to_send, self.selected_model);
                    self.request_tx.send(request_content).unwrap();
                    self.is_processing = true;
                    self.scroll_to_bottom();
                },
            );
        });

        // Central panel for messages
        egui::CentralPanel::default().show(ctx, |ui| {
            let message_list_handler = MessageListPanel;
            message_list_handler.show(
                ui,
                &self.messages,
                &mut self.markdown_cache,
                &mut self.pending_scroll,
                &mut self.current_scroll_offset,
                &mut self.last_scroll_area_height,
            );
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
                    self.scroll_to_bottom();
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
