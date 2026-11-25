use eframe::egui;

use crate::gui::app::WindowListApp;

pub(super) fn show(app: &WindowListApp, ui: &mut egui::Ui) {
    for error in app.state.errors.values() {
        egui::Frame::new()
            .fill(egui::Color32::from_rgb(61, 31, 37))
            .corner_radius(egui::CornerRadius::same(6))
            .inner_margin(egui::Margin::symmetric(10, 6))
            .show(ui, |ui| {
                ui.colored_label(egui::Color32::from_rgb(250, 166, 155), error);
            });
    }
}
