use eframe::egui;

use crate::gui::app::WindowListApp;

pub(super) fn show(app: &mut WindowListApp, ui: &mut egui::Ui, window: u32) {
    let saved_characters = app.state.persisted_characters.clone();
    let mut selected_character = None;
    let mut create_name = None;
    let mut delete_character = None;

    egui::Frame::new()
        .fill(egui::Color32::from_rgb(34, 29, 40))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(111, 85, 55)))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            show_heading(app, ui, window);
            ui.label(
                egui::RichText::new(
                    "Choisis un nom enregistré, ou fais un clic droit dessus pour le supprimer.",
                )
                .size(10.0)
                .color(egui::Color32::from_rgb(166, 157, 174)),
            );
            ui.add(
                egui::TextEdit::singleline(&mut app.state.association_query)
                    .desired_width(ui.available_width())
                    .hint_text("Rechercher ou saisir un nom"),
            );
            show_saved_names(
                app,
                ui,
                &saved_characters,
                &mut selected_character,
                &mut delete_character,
            );
            offer_create_name(app, ui, &saved_characters, &mut create_name);
        });

    apply_selection(
        app,
        window,
        selected_character,
        create_name,
        delete_character,
    );
    ui.add_space(6.0);
}

fn show_heading(app: &mut WindowListApp, ui: &mut egui::Ui, window: u32) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("ASSOCIER UN PERSONNAGE")
                .size(10.0)
                .strong()
                .color(egui::Color32::from_rgb(213, 177, 112)),
        );
        let position = app
            .state
            .windows
            .iter()
            .position(|candidate| *candidate == window)
            .unwrap_or(0)
            + 1;
        ui.label(
            egui::RichText::new(format!("pour le compte {position:02}"))
                .size(10.0)
                .color(egui::Color32::from_rgb(166, 157, 174)),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("Annuler").clicked() {
                app.state.association_window = None;
                app.state.association_query.clear();
            }
        });
    });
}

fn show_saved_names(
    app: &WindowListApp,
    ui: &mut egui::Ui,
    characters: &[crate::config::CharacterConfig],
    selected: &mut Option<String>,
    deleted: &mut Option<String>,
) {
    let query = app.state.association_query.trim().to_lowercase();
    let matches: Vec<_> = characters
        .iter()
        .filter(|character| query.is_empty() || character.name.to_lowercase().contains(&query))
        .collect();
    if matches.is_empty() {
        ui.label(
            egui::RichText::new("Aucun personnage enregistré ne correspond.")
                .size(10.0)
                .color(egui::Color32::from_rgb(166, 157, 174)),
        );
        return;
    }
    egui::ScrollArea::vertical()
        .max_height(96.0)
        .show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                for character in matches {
                    let response = ui.button(&character.name);
                    if response.clicked() {
                        *selected = Some(character.name.clone());
                    }
                    response.context_menu(|ui| {
                        if ui.button("Supprimer").clicked() {
                            *deleted = Some(character.name.clone());
                            ui.close();
                        }
                    });
                }
            });
        });
}

fn offer_create_name(
    app: &WindowListApp,
    ui: &mut egui::Ui,
    characters: &[crate::config::CharacterConfig],
    create_name: &mut Option<String>,
) {
    let query = app.state.association_query.trim();
    let already_exists = characters
        .iter()
        .any(|character| character.name.eq_ignore_ascii_case(query));
    if !query.is_empty()
        && !already_exists
        && ui
            .button(format!("Créer « {query} » et associer"))
            .clicked()
    {
        *create_name = Some(query.to_owned());
    }
}

fn apply_selection(
    app: &mut WindowListApp,
    window: u32,
    selected: Option<String>,
    created: Option<String>,
    deleted: Option<String>,
) {
    if let Some(name) = selected {
        app.state.assign_character(window, name);
        app.state.update_initiative_order_from_windows();
        close_dialog(app);
        app.refresh_window_assignments();
    } else if let Some(name) = created {
        app.state.create_character(window, name);
        app.state.update_initiative_order_from_windows();
        close_dialog(app);
        app.refresh_window_assignments();
    } else if let Some(name) = deleted {
        app.state.delete_character(&name);
        app.state.association_query.clear();
        app.refresh_window_assignments();
    }
}

fn close_dialog(app: &mut WindowListApp) {
    app.state.association_window = None;
    app.state.association_query.clear();
}
