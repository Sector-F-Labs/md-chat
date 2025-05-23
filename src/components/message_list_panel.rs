use egui;
use egui_commonmark::{CommonMarkCache, CommonMarkViewer};
use crate::app::ChatMessage; // Assuming ChatMessage is public and crate::app is correct

pub struct MessageListPanel;

impl MessageListPanel {
    pub fn show(
        &self,
        ui: &mut egui::Ui,
        messages: &Vec<ChatMessage>,
        markdown_cache: &mut CommonMarkCache,
        pending_scroll: &mut Option<f32>,
        current_scroll_offset: &mut f32,
        last_scroll_area_height: &mut f32,
    ) {
        let mut scroll_area = egui::ScrollArea::vertical();
        if let Some(offset) = *pending_scroll {
            scroll_area = scroll_area.scroll_offset(egui::Vec2::new(0.0, offset));
        }

        // The task mentions message_tops and copy_button_tops are likely unused.
        // If they were needed, they would be cleared here if passed as &mut.
        // For now, they are not part of the function signature.

        let output = scroll_area.show(ui, |ui| {
            *last_scroll_area_height = ui.available_height(); // Store visible height
            for message in messages {
                // let before = ui.cursor().top(); // Example if message_tops was needed
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        // let copy_button_top = ui.cursor().top(); // Example if copy_button_tops was needed
                        if ui
                            .button("Copy")
                            .on_hover_text("Copy entire message markdown")
                            .clicked()
                        {
                            ui.output_mut(|o| o.copied_text = message.content.clone());
                        }
                        // copy_button_tops.push(copy_button_top); // Example
                        ui.add_space(4.0); // Spacing between button and text
                        let viewer = CommonMarkViewer::new(); // As per original, created per message
                        viewer.show(ui, markdown_cache, &message.content);
                    });
                });
                // message_tops.push(before); // Example
                ui.add_space(8.0); // Spacing between messages
            }
        });

        // After rendering, update current_scroll_offset and clear pending_scroll
        *current_scroll_offset = output.state.offset.y;
        *pending_scroll = None;
    }
}
