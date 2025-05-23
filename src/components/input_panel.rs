use egui;
use crate::app::EditMode; // Assuming EditMode is public and crate::app is the correct path

pub struct InputPanel;

impl InputPanel {
    pub fn show(
        &self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        input_str: &mut String, // Renamed for clarity within component
        is_processing: &bool,
        edit_mode: &mut EditMode,
        send_message_callback: impl FnOnce(String),
    ) {
        ui.horizontal(|ui| {
            let available_width = ui.available_width();
            let button_width = 80.0; 
            let text_edit_height = 60.0;

            let text_edit_response = ui.add_sized(
                [available_width - button_width, text_edit_height],
                egui::TextEdit::multiline(input_str).hint_text("Enter your message..."),
            );

            match edit_mode {
                EditMode::Insert => {
                    text_edit_response.request_focus();
                }
                EditMode::Normal => {
                    if text_edit_response.has_focus() {
                        ctx.memory_mut(|mem| mem.surrender_focus(text_edit_response.id));
                        // If the user clicked into the text edit in normal mode, switch to insert
                        if text_edit_response.clicked() {
                            *edit_mode = EditMode::Insert;
                        }
                    }
                }
            }

            if *is_processing {
                // Spinner is typically placed near the button or where action occurs
                // For now, let's keep it simple. The original code placed it after the text_edit
                // and before the button if processing.
            }

            let send_button_enabled = *edit_mode == EditMode::Insert && !*is_processing;
            
            let send_button = egui::Button::new(if *is_processing { "..." } else { "Send" })
                .min_size(egui::vec2(button_width, text_edit_height));

            let mut send_triggered = false;

            if ui.add_enabled(send_button_enabled, send_button).clicked() {
                send_triggered = true;
            }

            // Handle Enter key press for sending, only if in Insert mode and text_edit has focus
            if send_button_enabled && 
               text_edit_response.has_focus() && 
               ui.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift) {
                send_triggered = true;
            }

            if send_triggered {
                if !input_str.trim().is_empty() {
                    send_message_callback(input_str.clone());
                    input_str.clear();
                    // Ensure focus remains or is regained by the text edit after sending
                    // This might be desired for quick follow-up messages.
                    // However, the original app.rs logic didn't explicitly do this after send.
                    // For now, we'll let egui handle default focus behavior.
                }
            }
            
            // The spinner can be shown next to the button or as an overlay
            // The original code showed it *instead* of the button text if processing inside the button
            // and also as a separate spinner *after* the text_edit if processing.
            // Let's show a spinner next to the button if processing.
            if *is_processing {
                 ui.add(egui::Spinner::new());
            }
        });
    }
}
