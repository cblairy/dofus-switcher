use eframe::egui;

use crate::gui::app::WindowListApp;
use crate::gui::app::launch::arrow_button;
use crate::gui::worker::WindowCommand;
use crate::runtime::WorkerKind;

pub(super) fn show(app: &mut WindowListApp, ui: &mut egui::Ui, index: usize, window: u32) {
    let focused = app.state.focused_window == Some(window);
    let (fill, stroke) = if focused {
        (
            egui::Color32::from_rgb(35, 47, 43),
            egui::Color32::from_rgb(83, 129, 99),
        )
    } else {
        (
            egui::Color32::from_rgb(27, 24, 35),
            egui::Color32::from_rgb(49, 43, 58),
        )
    };
    egui::Frame::new()
        .fill(fill)
        .stroke(egui::Stroke::new(1.0, stroke))
        .corner_radius(egui::CornerRadius::same(7))
        .inner_margin(egui::Margin::symmetric(9, 5))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                select_account(app, ui, index, window);
                show_identity(app, ui, window);
                show_actions(app, ui, index, window);
            });
        });
}

fn select_account(app: &mut WindowListApp, ui: &mut egui::Ui, index: usize, window: u32) {
    let label = egui::RichText::new(format!("{:02}", index + 1))
        .size(11.0)
        .strong();
    if ui
        .selectable_label(app.state.selected_window == Some(window), label)
        .clicked()
    {
        app.state.selected_window = Some(window);
        if app
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
}

fn show_identity(app: &WindowListApp, ui: &mut egui::Ui, window: u32) {
    ui.label(
        egui::RichText::new(format!("0x{window:08x}"))
            .size(10.0)
            .color(egui::Color32::from_rgb(132, 124, 143)),
    );
    let name = app
        .state
        .character_names
        .get(&window)
        .filter(|name| !name.trim().is_empty())
        .map(String::as_str)
        .unwrap_or("Aucun personnage lié");
    let color = if name == "Aucun personnage lié" {
        egui::Color32::from_rgb(153, 145, 161)
    } else {
        egui::Color32::from_rgb(239, 231, 216)
    };
    ui.add_sized(
        [100.0, 24.0],
        egui::Label::new(egui::RichText::new(name).color(color)).truncate(),
    )
    .on_hover_text(name);
}

fn show_actions(app: &mut WindowListApp, ui: &mut egui::Ui, index: usize, window: u32) {
    let name = app.state.character_names.get(&window);
    let assigned = name.is_some_and(|name| !name.trim().is_empty());
    let label = if assigned { "Changer" } else { "Associer…" };
    if ui
        .add_sized([78.0, 28.0], egui::Button::new(label))
        .clicked()
    {
        app.state.association_window = Some(window);
        app.state.association_query.clear();
        app.state.selected_window = Some(window);
    }
    show_shortcut_button(app, ui, window);
    if arrow_button(ui, true, index > 0) {
        move_window(app, window, -1);
    }
    if arrow_button(ui, false, index + 1 < app.state.windows.len()) {
        move_window(app, window, 1);
    }
}

fn show_shortcut_button(app: &mut WindowListApp, ui: &mut egui::Ui, window: u32) {
    if app.state.capture_window == Some(window) {
        if ui
            .add_sized([86.0, 28.0], egui::Button::new("Touche…"))
            .clicked()
        {
            app.state.capture_window = None;
        }
        return;
    }
    let label = app
        .state
        .shortcut_keys
        .get(&window)
        .cloned()
        .unwrap_or_else(|| "Raccourci".to_owned());
    if ui
        .add_sized([86.0, 28.0], egui::Button::new(label))
        .clicked()
    {
        app.state.capture_window = Some(window);
    }
}

fn move_window(app: &mut WindowListApp, window: u32, direction: isize) {
    app.state.move_window(window, direction);
    let _ = app
        .commands_tx
        .send(WindowCommand::SetWindowOrder(app.state.windows.clone()));
}
