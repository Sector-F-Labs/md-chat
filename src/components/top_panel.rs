use egui;
use crate::app::EditMode; // Assuming EditMode will be made public

pub struct TopPanel;

impl TopPanel {
    pub fn show(
        &self,
        ctx: &egui::Context,
        ui: &mut egui::Ui,
        dark_mode: &mut bool,
        is_processing: &bool,
        selected_model: &mut String,
        models: &Vec<String>,
        edit_mode: &EditMode,
        refresh_callback: impl FnOnce(),
    ) {
        ui.horizontal(|ui| {
            if ui.button(if *dark_mode { "🌙" } else { "☀" }).clicked() {
                *dark_mode = !*dark_mode;
                if *dark_mode {
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
                && !*is_processing
            {
                refresh_callback();
            }
            if *is_processing {
                ui.add(egui::Spinner::new());
            }
            egui::ComboBox::from_label("Model")
                .selected_text(selected_model)
                .show_ui(ui, |ui| {
                    for model in models {
                        ui.selectable_value(selected_model, model.clone(), model);
                    }
                });
            // Add a spacer to push the mode indicator to the right
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let (mode_text, bg_color, fg_color) = match edit_mode {
                    EditMode::Normal => ("NORMAL", egui::Color32::BLACK, egui::Color32::WHITE),
                    EditMode::Insert => ("INSERT", egui::Color32::WHITE, egui::Color32::BLACK),
                };
                ui.label(
                    egui::RichText::new(mode_text)
                        .strong()
                        .background_color(bg_color)
                        .color(fg_color)
                        .monospace()
                        .size(16.0),
                );
            });
        });
    }
}
