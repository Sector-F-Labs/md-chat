//! The window: Patina's frame and header bar, the conversation, the input
//! box and a status bar with the vim-style mode.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, Sender, channel};

use egui::{FontData, FontDefinitions, FontFamily};
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use patina::frame::{self, TitleBar};
use patina::ime;
use patina::skin::{self, Skin};
use serde::{Deserialize, Serialize};

use crate::config::AppConfig;
use crate::openai::{self, Role};

/// The skin used until the user picks one: stock egui, following the
/// desktop's light or dark setting.
const HOME_SKIN: &str = skin::DEFAULT_SKIN;

const SYSTEM_PROMPT: &str = "You are a helpful assistant. You can use markdown formatting in your responses.";

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum EditMode {
    Normal,
    Insert,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
pub struct ChatMessage {
    pub role: Role,
    pub content: String,
}

type HistoryResult = Result<Vec<ChatMessage>, String>;

/// What the header asks for, handled after it's drawn.
enum Action {
    Quit,
    RefreshHistory,
    RefreshModels,
    Export,
    SetSkin(String),
    ReloadSkins,
    OpenSkinsFolder,
    SystemTitleBar(bool),
}

pub struct App {
    config: AppConfig,
    skins: Vec<Skin>,
    skin_errors: Vec<String>,
    /// A skin for this run only, not saved (`MDCHAT_SKIN`, for screenshots).
    skin_override: Option<String>,
    /// `MDCHAT_SCREENSHOT`: where to save a screenshot, and whether it has
    /// been asked for.
    screenshot: Option<(PathBuf, bool)>,

    messages: Vec<ChatMessage>,
    input: String,
    /// The chat models the API has, fetched at start and from File → Refresh Models.
    models: Vec<String>,
    /// Empty until the list arrives.
    selected_model: String,
    models_rx: Option<Receiver<Result<Vec<String>, String>>>,
    models_error: Option<String>,
    /// What the last export did, for the status bar.
    export_status: Option<Result<String, String>>,
    markdown_cache: CommonMarkCache,
    request_tx: Sender<(String, String)>,
    response_rx: Receiver<Result<String, String>>,
    waiting_for_reply: bool,
    /// A history fetch in flight, and whether a failure should be shown
    /// (not at start-up, when Reservoir may simply not be running).
    history_rx: Option<(Receiver<HistoryResult>, bool)>,

    edit_mode: EditMode,
    pending_scroll: Option<f32>,
    scroll_to_end: bool,
    current_scroll_offset: f32,
    last_scroll_area_height: f32,
}

impl App {
    pub fn new(cc: &eframe::CreationContext<'_>, config: AppConfig) -> Self {
        patina::appearance::watch(&cc.egui_ctx);
        Self::with_context(&cc.egui_ctx, config)
    }

    fn with_context(egui_ctx: &egui::Context, config: AppConfig) -> Self {
        egui_ctx.set_fonts(fonts());
        let (skins, skin_errors) = load_skins();

        // API requests run on a thread of their own, so the window never waits on the network.
        let (request_tx, request_rx) = channel::<(String, String)>();
        let (response_tx, response_rx) = channel();
        let api_key = config.openai_api_key.clone().unwrap_or_default();
        let api_url = config.completions_url();
        let ctx = egui_ctx.clone();
        std::thread::spawn(move || {
            let rt = tokio::runtime::Runtime::new().expect("a tokio runtime");
            while let Ok((content, model)) = request_rx.recv() {
                let result = rt.block_on(openai::send_openai_request(&content, &model, &api_key, &api_url));
                if response_tx.send(result).is_err() {
                    break;
                }
                ctx.request_repaint();
            }
        });

        let screenshot = std::env::var_os("MDCHAT_SCREENSHOT").map(|p| (PathBuf::from(p), false));
        let mut app = Self {
            skins,
            skin_errors,
            skin_override: std::env::var("MDCHAT_SKIN").ok(),
            messages: vec![ChatMessage { role: Role::System, content: SYSTEM_PROMPT.into() }],
            input: String::new(),
            models: Vec::new(),
            selected_model: String::new(),
            models_rx: None,
            models_error: None,
            export_status: None,
            markdown_cache: CommonMarkCache::default(),
            request_tx,
            response_rx,
            waiting_for_reply: false,
            history_rx: None,
            edit_mode: EditMode::Insert,
            pending_scroll: None,
            scroll_to_end: false,
            current_scroll_offset: 0.0,
            last_scroll_area_height: 0.0,
            config,
            screenshot,
        };
        app.refresh_models(egui_ctx);
        if app.screenshot.is_some() {
            app.messages.extend(sample_conversation());
        } else {
            app.refresh_history(egui_ctx, false);
        }
        app.apply_skin(egui_ctx);
        app
    }

    fn custom_frame(&self) -> bool {
        !self.config.system_title_bar
    }

    /// The chosen skin, falling back to `HOME_SKIN` (a saved name may no
    /// longer exist), then Default.
    fn skin(&self) -> &Skin {
        let find = |name: &str| self.skins.iter().find(|s| s.name.eq_ignore_ascii_case(name));
        let chosen = self.skin_override.as_deref().or(self.config.skin.as_deref()).unwrap_or(HOME_SKIN);
        find(chosen).or_else(|| find(HOME_SKIN)).unwrap_or(&self.skins[0])
    }

    fn apply_skin(&self, ctx: &egui::Context) {
        self.skin().apply(ctx);
    }

    fn scroll_to_bottom(&mut self) {
        // Done by scrolling to the end of the content once it's laid out: a huge offset
        // instead pushes the layout to infinity and egui panics on a NaN size.
        self.scroll_to_end = true;
    }

    fn send_message(&mut self) {
        let text = self.input.trim().to_string();
        if text.is_empty() || self.waiting_for_reply || self.selected_model.is_empty() {
            return;
        }
        self.input.clear();
        self.messages.push(ChatMessage { role: Role::User, content: text.clone() });
        self.request_tx.send((text, self.selected_model.clone())).ok();
        self.waiting_for_reply = true;
        self.scroll_to_bottom();
    }

    /// Asks where to save, then writes the conversation there as CMF.
    fn export(&mut self) {
        if !crate::export::has_content(&self.messages) {
            return;
        }
        let Some(path) = rfd::FileDialog::new()
            .set_title("Export as CMF")
            .set_file_name(crate::export::file_name(&self.messages))
            .add_filter("Conversational Markdown", &["cmf", "md"])
            .save_file()
        else {
            return;
        };
        self.export_status = Some(match std::fs::write(&path, crate::export::to_cmf(&self.messages)) {
            Ok(()) => Ok(format!("Exported to {}", path.display())),
            Err(e) => Err(format!("Couldn't export to {}: {e}", path.display())),
        });
    }

    fn refresh_models(&mut self, ctx: &egui::Context) {
        if self.models_rx.is_some() {
            return;
        }
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        let url = self.config.models_url();
        let key = self.config.openai_api_key.clone().unwrap_or_default();
        std::thread::spawn(move || {
            let result = tokio::runtime::Runtime::new()
                .map_err(|e| e.to_string())
                .and_then(|rt| rt.block_on(openai::list_models(&url, &key)));
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.models_rx = Some(rx);
    }

    fn refresh_history(&mut self, ctx: &egui::Context, report_errors: bool) {
        if self.history_rx.is_some() {
            return;
        }
        let (tx, rx) = channel();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = tokio::runtime::Runtime::new()
                .map_err(|e| e.to_string())
                .and_then(|rt| rt.block_on(crate::history::fetch()));
            let _ = tx.send(result);
            ctx.request_repaint();
        });
        self.history_rx = Some((rx, report_errors));
    }

    /// Replies, history and the model list that have arrived since the last frame.
    fn receive(&mut self) {
        if let Some(rx) = &self.models_rx
            && let Ok(result) = rx.try_recv()
        {
            match result {
                Ok(models) => {
                    // The last model picked if it's still there, else the API's first.
                    let keep = [Some(&self.selected_model), self.config.model.as_ref()]
                        .into_iter()
                        .flatten()
                        .find(|m| models.contains(m))
                        .cloned();
                    self.selected_model = keep.or_else(|| models.first().cloned()).unwrap_or_default();
                    self.models_error = None;
                    self.models = models;
                }
                Err(error) => self.models_error = Some(format!("Can't list models at {}: {error}", self.config.api_url)),
            }
            self.models_rx = None;
        }
        if let Some((rx, report_errors)) = &self.history_rx
            && let Ok(result) = rx.try_recv()
        {
            match result {
                Ok(history) => {
                    // Keep the system message, replace the rest.
                    self.messages.truncate(1);
                    self.messages.extend(history);
                    self.scroll_to_bottom();
                }
                Err(error) if *report_errors => self
                    .messages
                    .push(ChatMessage { role: Role::System, content: format!("Error fetching history: {error}") }),
                Err(_) => {}
            }
            self.history_rx = None;
        }
        if let Ok(response) = self.response_rx.try_recv() {
            let message = match response {
                Ok(content) => ChatMessage { role: Role::Assistant, content },
                Err(error) => ChatMessage { role: Role::System, content: format!("Error: {error}") },
            };
            self.messages.push(message);
            self.waiting_for_reply = false;
            self.scroll_to_bottom();
        }
    }

    /// NORMAL mode's vim-like keys, and Esc into it.
    fn handle_keys(&mut self, ctx: &egui::Context) {
        match self.edit_mode {
            EditMode::Insert => {
                if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
                    self.edit_mode = EditMode::Normal;
                }
            }
            EditMode::Normal => {
                let (plain, shift) = ctx.input(|i| {
                    let pressed = |mods: egui::Modifiers| {
                        move |key| i.events.iter().any(|e| matches!(e, egui::Event::Key { key: k, pressed: true, modifiers, .. } if *k == key && *modifiers == mods))
                    };
                    let plain = pressed(egui::Modifiers::NONE);
                    let shift = pressed(egui::Modifiers::SHIFT);
                    (
                        [egui::Key::I, egui::Key::R, egui::Key::J, egui::Key::K, egui::Key::G].map(plain),
                        [egui::Key::J, egui::Key::K, egui::Key::G].map(shift),
                    )
                });
                let [i, r, j, k, g] = plain;
                let [big_j, big_k, big_g] = shift;
                let line = 60.0;
                let page = self.last_scroll_area_height;
                let offset = self.current_scroll_offset;
                if i {
                    self.edit_mode = EditMode::Insert;
                }
                if r {
                    self.refresh_history(ctx, true);
                }
                if j {
                    self.pending_scroll = Some(offset + line);
                }
                if k {
                    self.pending_scroll = Some((offset - line).max(0.0));
                }
                if big_j {
                    self.pending_scroll = Some(offset + page);
                }
                if big_k {
                    self.pending_scroll = Some((offset - page).max(0.0));
                }
                if g {
                    self.pending_scroll = Some(0.0);
                }
                if big_g {
                    self.scroll_to_bottom();
                }
            }
        }
    }

    /// Left side of the title bar: word menus (Patina's ADR-0006).
    fn menus(&self, ui: &mut egui::Ui, actions: &mut Vec<Action>) {
        ui.menu_button("File", |ui| {
            if ui.add(egui::Button::new("Refresh History").shortcut_text("R")).clicked() {
                actions.push(Action::RefreshHistory);
                ui.close();
            }
            let can_export = crate::export::has_content(&self.messages);
            if ui.add_enabled(can_export, egui::Button::new("Export as CMF…").shortcut_text("Ctrl+E")).clicked() {
                actions.push(Action::Export);
                ui.close();
            }
            ui.separator();
            if ui.button("Refresh Models").clicked() {
                actions.push(Action::RefreshModels);
                ui.close();
            }
            ui.separator();
            if ui.add(egui::Button::new("Quit").shortcut_text("Ctrl+Q")).clicked() {
                actions.push(Action::Quit);
                ui.close();
            }
        });
        ui.menu_button("View", |ui| {
            let mut system = self.config.system_title_bar;
            if ui.checkbox(&mut system, "System Title Bar").changed() {
                actions.push(Action::SystemTitleBar(system));
            }
        });
        ui.menu_button("Skins", |ui| {
            let current = self.skin().name.clone();
            for skin in &self.skins {
                if ui.radio(skin.name == current, &skin.name).clicked() {
                    actions.push(Action::SetSkin(skin.name.clone()));
                }
            }
            ui.separator();
            if ui.button("Reload skins").clicked() {
                actions.push(Action::ReloadSkins);
            }
            if ui.button("Open skins folder").clicked() {
                actions.push(Action::OpenSkinsFolder);
                ui.close();
            }
            for error in &self.skin_errors {
                ui.colored_label(ui.visuals().error_fg_color, error);
            }
        });
    }

    fn handle(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Action::RefreshHistory => self.refresh_history(ctx, true),
            Action::RefreshModels => self.refresh_models(ctx),
            Action::Export => self.export(),
            Action::SetSkin(name) => {
                self.skin_override = None;
                self.config.skin = Some(name);
                self.config.save();
                self.apply_skin(ctx);
            }
            Action::ReloadSkins => {
                (self.skins, self.skin_errors) = load_skins();
                self.apply_skin(ctx);
            }
            Action::OpenSkinsFolder => {
                if let Some(dir) = skin::shared_dir() {
                    let _ = std::fs::create_dir_all(&dir);
                    let _ = patina::open::path(&dir);
                }
            }
            Action::SystemTitleBar(on) => {
                self.config.system_title_bar = on;
                self.config.save();
                ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(on));
            }
        }
    }

    /// The bottom of the window: the vim-style mode, and what's going on.
    fn status_bar(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            let v = ui.visuals();
            let (text, bg, fg) = match self.edit_mode {
                EditMode::Normal => (" NORMAL ", v.selection.bg_fill, v.selection.stroke.color),
                EditMode::Insert => (" INSERT ", v.extreme_bg_color, v.text_color()),
            };
            ui.label(egui::RichText::new(text).monospace().strong().background_color(bg).color(fg));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(error) = &self.models_error {
                    ui.colored_label(ui.visuals().error_fg_color, error);
                } else if self.waiting_for_reply {
                    ui.label(format!("Waiting for {}", self.selected_model));
                    ui.add(egui::Spinner::new());
                } else if self.history_rx.is_some() {
                    ui.label("Fetching history");
                    ui.add(egui::Spinner::new());
                } else if let Some(status) = &self.export_status {
                    match status {
                        Ok(done) => ui.label(done),
                        Err(error) => ui.colored_label(ui.visuals().error_fg_color, error),
                    };
                }
            });
        });
    }

    fn input_panel(&mut self, ui: &mut egui::Ui) {
        let button_width = 80.0;
        let height = 60.0;
        ui.horizontal(|ui| {
            let send_key = egui::KeyboardShortcut::new(egui::Modifiers::NONE, egui::Key::Enter);
            let text_edit = ui.add_sized(
                [ui.available_width() - button_width - ui.spacing().item_spacing.x, height],
                egui::TextEdit::multiline(&mut self.input)
                    .hint_text("Message (Enter sends, Shift+Enter for a new line)")
                    // Shift+Enter adds a line; plain Enter sends, below.
                    .return_key(egui::KeyboardShortcut::new(egui::Modifiers::SHIFT, egui::Key::Enter)),
            );
            match self.edit_mode {
                // Only when it's lost: each request interrupts the IME, which on
                // Wayland answers with an event, and so another frame.
                EditMode::Insert if !text_edit.has_focus() => text_edit.request_focus(),
                EditMode::Insert => {}
                EditMode::Normal => {
                    // Clicking the input box goes back to INSERT mode.
                    if text_edit.has_focus() || text_edit.clicked() {
                        self.edit_mode = EditMode::Insert;
                    }
                }
            }
            let enter = self.edit_mode == EditMode::Insert && ui.input_mut(|i| i.consume_shortcut(&send_key));
            let can_send = !self.waiting_for_reply && !self.selected_model.is_empty() && !self.input.trim().is_empty();
            let label = if self.waiting_for_reply { "…" } else { "Send" };
            let send = ui.add_enabled(can_send, egui::Button::new(label).min_size(egui::vec2(button_width, height)));
            if send.clicked() || enter {
                self.send_message();
            }
        });
    }

    fn conversation(&mut self, ui: &mut egui::Ui) {
        let mut scroll_area = egui::ScrollArea::vertical().auto_shrink(false);
        if let Some(offset) = self.pending_scroll.take() {
            scroll_area = scroll_area.vertical_scroll_offset(offset);
        }
        let output = scroll_area.show(ui, |ui| {
            self.last_scroll_area_height = ui.clip_rect().height();
            for message in &self.messages {
                ui.group(|ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal_top(|ui| {
                        if ui.button("Copy").on_hover_text("Copy the message's markdown").clicked() {
                            ui.ctx().copy_text(message.content.clone());
                        }
                        ui.add_space(4.0);
                        ui.vertical(|ui| {
                            CommonMarkViewer::new().show(ui, &mut self.markdown_cache, &message.content);
                        });
                    });
                });
                ui.add_space(8.0);
            }
            if std::mem::take(&mut self.scroll_to_end) {
                ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
            }
        });
        self.current_scroll_offset = output.state.offset.y;
    }

    fn take_screenshot(&mut self, ctx: &egui::Context) {
        let Some((path, requested)) = &mut self.screenshot else {
            return;
        };
        ctx.request_repaint();
        let image = ctx.input(|i| {
            i.raw.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            match crate::screenshot::save(&image, path) {
                Ok(()) => eprintln!("saved {}", path.display()),
                Err(e) => eprintln!("screenshot failed: {e}"),
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        } else if !*requested && ctx.input(|i| i.time) > 1.0 {
            *requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.show(ui);
    }

    fn clear_color(&self, visuals: &egui::Visuals) -> [f32; 4] {
        frame::clear_color(visuals, self.custom_frame())
    }

    fn raw_input_hook(&mut self, ctx: &egui::Context, raw_input: &mut egui::RawInput) {
        frame::release_grab(ctx, raw_input);
        ime::drop_repeated_preedits(ctx, raw_input);
    }
}

impl App {
    fn show(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        self.receive();
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::Q)) {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::COMMAND, egui::Key::E)) {
            self.export();
        }
        self.handle_keys(&ctx);

        // Every frame: Default's colours follow the desktop's light/dark setting.
        let style = self.skin().frame_style(ui.visuals());
        let custom = self.custom_frame();
        let radius = if custom { frame::corner_radius(&ctx, style.radius) } else { 0 };

        let mut actions = Vec::new();
        let mut model = self.selected_model.clone();
        let mut export_clicked = false;
        egui::Panel::top("header").frame(frame::header_frame(ui.style(), &style, radius)).show(ui, |ui| {
            TitleBar::new("MD-Chat", &style).custom(custom).menu_bar(true).show(
                ui,
                |ui| self.menus(ui, &mut actions),
                |ui| {
                    // Right to left: Export by the window buttons, then the model.
                    let can_export = crate::export::has_content(&self.messages);
                    let export = ui.add_enabled(can_export, egui::Button::new("Export"));
                    export_clicked = export.on_hover_text("Save the conversation as CMF (Ctrl+E)").clicked();
                    model_picker(ui, &self.models, self.models_rx.is_some(), &mut model);
                },
            );
        });
        if export_clicked {
            actions.push(Action::Export);
        }
        if model != self.selected_model {
            self.config.model = Some(model.clone());
            self.config.save();
            self.selected_model = model;
        }
        for action in actions {
            self.handle(&ctx, action);
        }

        // The status bar holds both bottom corners.
        egui::Panel::bottom("status")
            .frame(egui::Frame::side_top_panel(ui.style()).corner_radius(egui::CornerRadius {
                sw: radius,
                se: radius,
                ..Default::default()
            }))
            .show(ui, |ui| self.status_bar(ui));
        egui::Panel::bottom("input").show(ui, |ui| {
            ui.add_space(4.0);
            self.input_panel(ui);
            ui.add_space(2.0);
        });
        egui::CentralPanel::default().show(ui, |ui| self.conversation(ui));

        if custom {
            frame::edges(&ctx, radius, style.border);
        }
        self.take_screenshot(&ctx);
    }
}

/// On the right of the title bar: the model that writes the replies, the
/// app's output.
fn model_picker(ui: &mut egui::Ui, models: &[String], loading: bool, selected: &mut String) {
    if models.is_empty() {
        ui.label(if loading { "Loading models…" } else { "No models" });
        return;
    }
    egui::ComboBox::from_id_salt("model").selected_text(selected.as_str()).show_ui(ui, |ui| {
        for model in models {
            ui.selectable_value(selected, model.clone(), model);
        }
    });
    ui.label("Model");
}

/// Patina's built-ins and the shared skins folder.
fn load_skins() -> (Vec<Skin>, Vec<String>) {
    let dirs: Vec<_> = skin::shared_dir().into_iter().collect();
    skin::load_all(skin::BUILT_IN, &dirs)
}

/// Lexend for text, with monochrome Noto Emoji as a fallback; code stays in
/// egui's monospace font (ADR-0002).
fn fonts() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    let lexend = include_bytes!("../assets/Lexend-VariableFont_wght.ttf");
    let emoji = include_bytes!("../assets/NotoEmoji-VariableFont_wght.ttf");
    fonts.font_data.insert("lexend".into(), FontData::from_static(lexend).into());
    fonts.font_data.insert("emoji".into(), FontData::from_static(emoji).into());
    let proportional = fonts.families.entry(FontFamily::Proportional).or_default();
    proportional.insert(0, "lexend".into());
    proportional.push("emoji".into());
    fonts.families.entry(FontFamily::Monospace).or_default().push("emoji".into());
    fonts
}

/// The conversation shown in screenshot mode.
fn sample_conversation() -> Vec<ChatMessage> {
    vec![
        ChatMessage { role: Role::User, content: "Show me what markdown you can do.".into() },
        ChatMessage { role: Role::Assistant, content: include_str!("../example.md").into() },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs one frame of the app in an 800×600 window.
    fn frame(ctx: &egui::Context, app: &mut App, events: Vec<egui::Event>) {
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))),
            events,
            ..Default::default()
        };
        let _ = ctx.run_ui(input, |ui| app.show(ui));
    }

    #[test]
    fn sending_with_a_long_conversation_scrolls_to_the_end_without_panicking() {
        let ctx = egui::Context::default();
        // Nothing listens on port 9: the model list and the reply fail fast, offline.
        let config = AppConfig { api_url: "http://127.0.0.1:9".into(), ..Default::default() };
        let mut app = App::with_context(&ctx, config);
        app.messages.extend(sample_conversation());
        app.selected_model = "test-model".into();
        frame(&ctx, &mut app, vec![]);

        frame(&ctx, &mut app, vec![egui::Event::Text("Hello".into())]);
        let enter = egui::Event::Key {
            key: egui::Key::Enter,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: egui::Modifiers::NONE,
        };
        frame(&ctx, &mut app, vec![enter]);
        assert!(matches!(app.messages.last(), Some(ChatMessage { role: Role::User, content }) if content == "Hello"));
        // The reply (an error here) scrolls to the end again.
        for _ in 0..500 {
            frame(&ctx, &mut app, vec![]);
            if !app.waiting_for_reply {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert!(!app.waiting_for_reply);
        frame(&ctx, &mut app, vec![]);
    }
}
