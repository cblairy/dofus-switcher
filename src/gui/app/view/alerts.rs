use eframe::egui;

use crate::gui::app::WindowListApp;

pub(super) fn show(app: &WindowListApp, ui: &mut egui::Ui) {
    if let Some(error) = &app.state.config_error {
        show_error(ui, error);
    }
    for error in app.state.errors.values() {
        show_error(ui, error);
    }
}

fn show_error(ui: &mut egui::Ui, error: &str) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(61, 31, 37))
        .corner_radius(egui::CornerRadius::same(6))
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.colored_label(egui::Color32::from_rgb(250, 166, 155), error);
        });
}
