use eframe::egui;

use crate::gui::app::WindowListApp;
use crate::gui::worker::WindowCommand;
use crate::runtime::WorkerKind;

pub(super) fn show(app: &mut WindowListApp, ui: &mut egui::Ui) {
    ui.add_space(8.0);
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(27, 24, 35))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 46, 62)))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                draw_status(app, ui);
                ui.separator();
                draw_controls(app, ui);
            });
        });
}

fn draw_status(app: &WindowListApp, ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("LECTURE DU TOUR · OCR")
                .size(10.0)
                .strong()
                .color(egui::Color32::from_rgb(202, 167, 104)),
        );
        let text = if let Some(window) = app.state.ocr_target {
            format!("Personnage détecté : {}", app.state.character_label(window))
        } else if app.state.ocr_region.is_some() {
            "En attente d’un début de tour…".to_owned()
        } else {
            "Zone inactive · place-la sur la vignette du début de tour.".to_owned()
        };
        ui.label(
            egui::RichText::new(text)
                .size(10.0)
                .color(egui::Color32::from_rgb(166, 157, 174)),
        );
    });
}

fn draw_controls(app: &mut WindowListApp, ui: &mut egui::Ui) {
    if app.state.ocr_region.is_some()
        && ui
            .button("Désactiver")
            .on_hover_text("Mettre en pause la lecture OCR.")
            .clicked()
    {
        app.state.ocr_region = None;
        app.state.ocr_text.clear();
        app.state.ocr_target = None;
        app.state.ocr_candidate = None;
        app.state.ocr_candidate_hits = 0;
        app.state.ocr_candidate_switched = false;
        app.worker.set_ocr_region(None);
    }
    let action = if app.state.ocr_region.is_some() {
        "Repositionner"
    } else {
        "Placer la zone"
    };
    let hint = if app.state.ocr_region.is_some() {
        "Repositionner ou redimensionner la zone OCR sur la vignette de début de tour."
    } else {
        "Choisir une fenêtre Dofus puis placer la zone OCR sur la vignette de début de tour."
    };
    if ui.button(action).on_hover_text(hint).clicked() && app.begin_ocr_calibration(ui.ctx()) {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
    if let Some(window) = app.state.ocr_target {
        switch_to_detected_window(app, ui, window);
    }
}

fn switch_to_detected_window(app: &mut WindowListApp, ui: &mut egui::Ui, window: u32) {
    if ui
        .button("Basculer")
        .on_hover_text("Activer la fenêtre du personnage détecté.")
        .clicked()
        && app
            .commands_tx
            .send(WindowCommand::Activate(window))
            .is_err()
    {
        app.state.errors.insert(
            WorkerKind::Activation,
            "Le moniteur de fenêtres est indisponible.".to_owned(),
        );
    }
}
