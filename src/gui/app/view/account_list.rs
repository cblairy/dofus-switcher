use eframe::egui;

use crate::gui::app::WindowListApp;

use super::account_row;
use super::association;

pub(super) fn show(app: &mut WindowListApp, ui: &mut egui::Ui) {
    if app.state.windows.is_empty() {
        show_empty_state(ui);
        return;
    }
    show_order_header(app, ui);
    if let Some(window) = app.state.association_window {
        association::show(app, ui, window);
    }
    for (index, window) in app.state.windows.clone().into_iter().enumerate() {
        account_row::show(app, ui, index, window);
    }
}

fn show_empty_state(ui: &mut egui::Ui) {
    ui.add_space(12.0);
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(27, 24, 35))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 46, 62)))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(16, 18))
        .show(ui, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Aucun compte Dofus détecté")
                        .size(15.0)
                        .strong()
                        .color(egui::Color32::from_rgb(223, 211, 192)),
                );
                ui.label(
                    egui::RichText::new(
                        "Lance tes fenêtres de jeu pour les ajouter à l’ordre de tour.",
                    )
                    .size(11.0)
                    .color(egui::Color32::from_rgb(166, 157, 174)),
                );
            });
        });
}

fn show_order_header(app: &WindowListApp, ui: &mut egui::Ui) {
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("ORDRE DE TOUR")
                .size(10.0)
                .strong()
                .color(egui::Color32::from_rgb(202, 167, 104)),
        );
        ui.separator();
        let unassigned = app
            .state
            .windows
            .iter()
            .filter(|window| {
                app.state
                    .character_names
                    .get(window)
                    .is_none_or(|name| name.trim().is_empty())
            })
            .count();
        if unassigned > 0 {
            ui.label(
                egui::RichText::new(format!("{unassigned} À ASSOCIER"))
                    .size(10.0)
                    .strong()
                    .color(egui::Color32::from_rgb(213, 168, 94)),
            );
        }
        show_focus_state(app, ui);
    });
}

fn show_focus_state(app: &WindowListApp, ui: &mut egui::Ui) {
    let (text, color) = if let Some(window) = app.state.focused_window {
        (
            format!("ACTIF · {}", app.state.character_label(window)),
            egui::Color32::from_rgb(144, 215, 176),
        )
    } else {
        (
            "AUCUN COMPTE FOCALISÉ".to_owned(),
            egui::Color32::from_rgb(166, 157, 174),
        )
    };
    ui.label(egui::RichText::new(text).size(10.0).color(color));
}
