// main.rs
use eframe::egui;
use egui_commonmark::{CommonMarkViewer, CommonMarkCache};
use std::sync::mpsc::{channel, Receiver, Sender};

const APP_NAME: &str = "OpenAI Chat";

mod openai;
use openai::{ChatCompletionMessage, ChatCompletionRequest, ChatCompletionResponse, Role};

#[derive(Debug)]
struct ChatMessage {
    role: Role,
    content: String,
}

struct MyApp {
    dark_mode: bool,
    messages: Vec<ChatMessage>,
    input: String,
    http_client: reqwest::Client,
    response_rx: Receiver<Result<String, String>>,
    request_tx: Sender<String>,
    is_processing: bool,
    markdown_cache: CommonMarkCache,
}

impl MyApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Initialize HTTP client
        let http_client = reqwest::Client::new();
        
        // Set up channels for async communication
        let (request_tx, request_rx): (Sender<String>, Receiver<String>) = channel();
        let (response_tx, response_rx): (Sender<Result<String, String>>, Receiver<Result<String, String>>) = channel();

        // Spawn background thread for handling API requests
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().unwrap();
            while let Ok(message) = request_rx.recv() {
                let tx = response_tx.clone();
                rt.block_on(async {
                    let result = openai::send_openai_request(&message).await;
                    tx.send(result).unwrap();
                });
            }
        });

        let mut app = Self {
            dark_mode: true,
            messages: Vec::new(),
            input: String::new(),
            http_client,
            response_rx,
            request_tx,
            is_processing: false,
            markdown_cache: CommonMarkCache::default(),
        };

        // Add initial system message
        app.messages.push(ChatMessage {
            role: Role::System,
            content: "You are a helpful assistant. You can use markdown formatting in your responses.".to_string(),
        });

        app
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
            });
        });

        // Bottom panel for input
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let text_edit = ui.text_edit_multiline(&mut self.input);
                text_edit.request_focus();

                if ui.button(if self.is_processing { "..." } else { "Send" }).clicked() 
                    || (ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift)) 
                {
                    if !self.input.trim().is_empty() && !self.is_processing {
                        let message = std::mem::take(&mut self.input);
                        self.messages.push(ChatMessage {
                            role: Role::User,
                            content: message.clone(),
                        });
                        self.request_tx.send(message).unwrap();
                        self.is_processing = true;
                    }
                }
            });
        });

        // Central panel for messages
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                for message in &self.messages {
                    ui.group(|ui| {
                        let mut viewer = CommonMarkViewer::new();
                        viewer.show(ui, &mut self.markdown_cache, &message.content);
                    });
                    ui.add_space(8.0);
                }
            });
        });

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

fn main() {
    let native_options = eframe::NativeOptions {
        persist_window: true,
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_position([50.0, 50.0])
            .with_app_id(APP_NAME)
            .with_min_inner_size([400.0, 300.0])
            .with_resizable(true),
        centered: true,
        ..Default::default()
    };

    eframe::run_native(
        APP_NAME,
        native_options,
        Box::new(|cc| Ok(Box::new(MyApp::new(cc)))),
    )
    .expect("Failed to launch application");
}
