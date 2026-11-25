use eframe::egui;

use crate::gui::app::WindowListApp;

pub(super) fn show(app: &mut WindowListApp, ctx: &egui::Context) {
    if !app.help_open {
        return;
    }

    egui::Window::new("Guide rapide")
        .id(egui::Id::new("quick-help"))
        .open(&mut app.help_open)
        .collapsible(false)
        .resizable(true)
        .default_size([410.0, 470.0])
        .show(ctx, |ui| {
            egui::ScrollArea::vertical()
                .max_height(410.0)
                .show(ui, |ui| {
                    ui.label(
                        egui::RichText::new(
                            "Prépare tes comptes une fois, puis passe de l’un à l’autre en jeu.",
                        )
                        .color(egui::Color32::from_rgb(190, 181, 198)),
                    );
                    ui.add_space(10.0);
                    step(
                        ui,
                        "1",
                        "Associer les personnages",
                        "Clique sur « Associer… » pour choisir un nom enregistré ou en créer un. Les noms sont mémorisés et l’association est restaurée automatiquement quand les fenêtres sont reconnues. Sinon, associe-les à nouveau.",
                    );
                    step(
                        ui,
                        "2",
                        "Régler l’ordre et les raccourcis",
                        "Les flèches déplacent un compte dans l’ordre de tour. Clique sur « Précédent » ou « Suivant » pour enregistrer la touche qui fera défiler tes fenêtres. Le bouton « Raccourci » attribue une touche directe à un compte.",
                    );
                    step(
                        ui,
                        "3",
                        "Préparer l’affichage de Dofus",
                        "Dans Options > Jeu > Général, active « Afficher l’image de la classe comme picto de la fenêtre ». Cela aide à distinguer tes fenêtres. L’OCR lit le nom affiché dans le jeu, pas cette icône.",
                    );
                    screenshot(
                        ui,
                        &app.help_images.window_icon_setting,
                        "Option à activer dans les paramètres de Dofus",
                    );
                    step(
                        ui,
                        "4",
                        "Placer et cadrer la zone OCR",
                        "Sélectionne un compte, puis clique sur « Placer la zone ». Déplace la barre du cadre sur le nom du personnage qui apparaît au début de son tour. Garde le nom entier dans la zone et évite d’inclure le niveau ou le décor. La poignée en bas à droite redimensionne le cadre ; Entrée confirme et Échap annule. La position est mémorisée.",
                    );
                    screenshot(
                        ui,
                        &app.help_images.turn_character_name,
                        "Exemple : centre le cadre sur le nom du personnage, ici « Cornedrue ».",
                    );
                    step(
                        ui,
                        "OCR",
                        "Détection du début de tour",
                        "Quand le nom lu correspond à un personnage associé, Dofus Switcher active sa fenêtre. L’état affiché indique si l’OCR attend ou a trouvé un personnage.",
                    );
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Astuce : survole les boutons pour afficher leur aide.",
                        )
                        .size(11.0)
                        .color(egui::Color32::from_rgb(202, 167, 104)),
                    );
                });
        });
}

fn screenshot(ui: &mut egui::Ui, texture: &egui::TextureHandle, caption: &str) {
    ui.add_space(2.0);
    ui.add(egui::Image::new(texture).max_width(ui.available_width()));
    ui.label(
        egui::RichText::new(caption)
            .size(10.0)
            .color(egui::Color32::from_rgb(166, 157, 174)),
    );
    ui.add_space(6.0);
}

fn step(ui: &mut egui::Ui, number: &str, title: &str, description: &str) {
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(27, 24, 35))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(52, 46, 62)))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(number)
                        .strong()
                        .color(egui::Color32::from_rgb(202, 167, 104)),
                );
                ui.label(
                    egui::RichText::new(title)
                        .strong()
                        .color(egui::Color32::from_rgb(237, 207, 153)),
                );
            });
            ui.label(
                egui::RichText::new(description)
                    .size(11.0)
                    .color(egui::Color32::from_rgb(190, 181, 198)),
            );
        });
    ui.add_space(6.0);
}
