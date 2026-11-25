use eframe::egui;

pub(super) struct HelpImages {
    pub(super) window_icon_setting: egui::TextureHandle,
    pub(super) turn_character_name: egui::TextureHandle,
}

impl HelpImages {
    pub(super) fn load(ctx: &egui::Context) -> Self {
        Self {
            window_icon_setting: load_texture(
                ctx,
                "help-dofus-window-icon-setting",
                include_bytes!("../../../assets/help/dofus-option-fenetre.png"),
            ),
            turn_character_name: load_texture(
                ctx,
                "help-turn-character-name",
                include_bytes!("../../../assets/help/vignette-debut-tour.png"),
            ),
        }
    }
}

fn load_texture(ctx: &egui::Context, name: &str, bytes: &[u8]) -> egui::TextureHandle {
    let image = image::load_from_memory(bytes)
        .expect("Les captures d’aide OCR intégrées doivent être valides")
        .into_rgba8();
    ctx.load_texture(
        name,
        egui::ColorImage::from_rgba_unmultiplied(
            [image.width() as usize, image.height() as usize],
            image.as_raw(),
        ),
        egui::TextureOptions::LINEAR,
    )
}
