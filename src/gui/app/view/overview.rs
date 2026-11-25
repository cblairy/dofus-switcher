use eframe::egui;

use crate::gui::app::WindowListApp;

pub(super) fn show(app: &WindowListApp, ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        draw_emblem(ui);
        ui.vertical(|ui| {
            ui.label(
                egui::RichText::new("ORDRE D’INITIATIVE")
                    .size(18.0)
                    .strong()
                    .color(egui::Color32::from_rgb(237, 207, 153)),
            );
            ui.label(
                egui::RichText::new("GESTION DES COMPTES · SWITCH RAPIDE")
                    .size(10.0)
                    .color(egui::Color32::from_rgb(166, 157, 174)),
            );
        });
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            draw_account_count(app, ui);
        });
    });
    ui.add_space(10.0);
    ui.separator();
}

fn draw_emblem(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(34.0, 34.0), egui::Sense::hover());
    let center = rect.center();
    ui.painter()
        .circle_filled(center, 15.0, egui::Color32::from_rgb(48, 38, 48));
    ui.painter().circle_stroke(
        center,
        14.0,
        egui::Stroke::new(1.0, egui::Color32::from_rgb(206, 160, 83)),
    );
    let gem = [
        egui::pos2(center.x, center.y - 8.0),
        egui::pos2(center.x + 5.0, center.y),
        egui::pos2(center.x, center.y + 8.0),
        egui::pos2(center.x - 5.0, center.y),
    ];
    ui.painter().add(egui::Shape::convex_polygon(
        gem.to_vec(),
        egui::Color32::from_rgb(100, 185, 151),
        egui::Stroke::new(1.0, egui::Color32::from_rgb(224, 190, 124)),
    ));
}

fn draw_account_count(app: &WindowListApp, ui: &mut egui::Ui) {
    let count = app.state.windows.len();
    let label = if count == 1 {
        "1 COMPTE".to_owned()
    } else {
        format!("{count} COMPTES")
    };
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(37, 46, 43))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(67, 107, 87)))
        .corner_radius(egui::CornerRadius::same(12))
        .inner_margin(egui::Margin::symmetric(10, 5))
        .show(ui, |ui| {
            ui.label(
                egui::RichText::new(label)
                    .size(10.0)
                    .strong()
                    .color(egui::Color32::from_rgb(144, 215, 176)),
            );
        });
}
