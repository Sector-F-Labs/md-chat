// main.rs
use eframe::egui;
use egui_commonmark::{CommonMarkViewer, CommonMarkCache};
use std::sync::mpsc::{channel, Receiver, Sender};
use eframe::egui::{IconData, ViewportBuilder};
use std::sync::Arc;
use image;

const APP_NAME: &str = "MD-Chat";

// Add your preferred models here
const AVAILABLE_MODELS: &[&str] = &[
    "gpt-4.1",
    "gpt-4o-mini",
    "gpt-4o",
];

mod openai;
use openai::Role;

#[allow(dead_code)]
struct ChatMessage {
    role: Role,
    content: String,
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
}

#[allow(dead_code)]
impl MyApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
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
                let (content, model) = message.split_once('\0').unwrap();
                rt.block_on(async {
                    let result = openai::send_openai_request(content, model).await;
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
            selected_model: AVAILABLE_MODELS[0].to_string(),
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
                
                ui.separator();
                
                egui::ComboBox::from_label("Model")
                    .selected_text(&self.selected_model)
                    .show_ui(ui, |ui| {
                        for model in AVAILABLE_MODELS {
                            ui.selectable_value(&mut self.selected_model, model.to_string(), *model);
                        }
                    });
            });
        });

        // Bottom panel for input
        egui::TopBottomPanel::bottom("bottom_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                let text_edit = ui.text_edit_multiline(&mut self.input);
                text_edit.request_focus();

                if self.is_processing {
                    ui.add(egui::Spinner::new());
                }

                if ui.button(if self.is_processing { "..." } else { "Send" }).clicked() 
                    || (ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift)) 
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
                }
            });
        });

        // Central panel for messages
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().stick_to_bottom(true).show(ui, |ui| {
                for message in &self.messages {
                    ui.group(|ui| {
                        ui.horizontal(|ui| {
                            // Copy button first, aligned to top right
                            if ui.button("Copy").on_hover_text("Copy entire message markdown").clicked() {
                                ui.output_mut(|o| o.copied_text = message.content.clone());
                            }
                            // Add some spacing between the button and the text
                            ui.add_space(4.0);
                            let viewer = CommonMarkViewer::new();
                            viewer.show(ui, &mut self.markdown_cache, &message.content);
                        });
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
