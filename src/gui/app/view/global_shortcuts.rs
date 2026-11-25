use eframe::egui;

use crate::gui::app::WindowListApp;
use crate::gui::state::GlobalAction;

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
                draw_label(ui);
                ui.separator();
                draw_shortcut(app, ui, GlobalAction::Previous);
                draw_shortcut(app, ui, GlobalAction::Next);
            });
        });
}

fn draw_label(ui: &mut egui::Ui) {
    ui.vertical(|ui| {
        ui.label(
            egui::RichText::new("CYCLE DE COMPTES")
                .size(10.0)
                .strong()
                .color(egui::Color32::from_rgb(202, 167, 104)),
        );
        ui.label(
            egui::RichText::new("Raccourcis globaux")
                .size(11.0)
                .color(egui::Color32::from_rgb(166, 157, 174)),
        );
    });
}

fn draw_shortcut(app: &mut WindowListApp, ui: &mut egui::Ui, action: GlobalAction) {
    let capture_enabled = !app
        .state
        .errors
        .contains_key(&crate::runtime::WorkerKind::HotkeyCapture);
    if app.state.capture_global == Some(action) {
        if ui
            .add_enabled(
                capture_enabled,
                egui::Button::new("Appuyez sur une touche…"),
            )
            .on_hover_text("Appuie sur la touche souhaitée, ou clique ici pour annuler.")
            .clicked()
        {
            app.state.capture_global = None;
        }
        return;
    }
    let (label, key) = match action {
        GlobalAction::Previous => (
            "Précédent",
            app.state.previous_key.as_deref().unwrap_or("—"),
        ),
        GlobalAction::Next => ("Suivant", app.state.next_key.as_deref().unwrap_or("—")),
    };
    if ui
        .add_enabled(
            capture_enabled,
            egui::Button::new(format!("{label}   {key}")),
        )
        .on_hover_text(if capture_enabled {
            "Clique puis appuie sur une touche pour changer ce raccourci."
        } else {
            "Écoute clavier indisponible : consulte le message d’état en haut de la fenêtre."
        })
        .clicked()
    {
        app.state.capture_global = Some(action);
    }
}
