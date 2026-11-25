use super::WindowListApp;
use crate::gui::worker::WorkerHandle;
use eframe::egui;

pub(crate) fn run() -> eframe::Result<()> {
    let worker = WorkerHandle::start();
    let icon = image::load_from_memory(include_bytes!("../../../assets/icons/dofus-switcher.png"))
        .expect("L’icône de fenêtre intégrée doit être une image valide")
        .into_rgba8();
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([520.0, 440.0])
            .with_min_inner_size([520.0, 220.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_has_shadow(false)
            .with_icon(egui::IconData {
                width: icon.width(),
                height: icon.height(),
                rgba: icon.into_raw(),
            }),
        ..Default::default()
    };
    eframe::run_native(
        "Dofus Switcher",
        options,
        Box::new(move |creation_context| {
            apply_dofus_style(&creation_context.egui_ctx);
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "arrow_icons".to_owned(),
                egui::FontData::from_static(include_bytes!(
                    "../../../assets/fonts/NotoSansSymbols-Arrows.ttf"
                ))
                .into(),
            );
            fonts.families.insert(
                egui::FontFamily::Name("arrow_icons".into()),
                vec!["arrow_icons".to_owned()],
            );
            creation_context.egui_ctx.set_fonts(fonts);
            let icon_image =
                image::load_from_memory(include_bytes!("../../../assets/icons/dofus-switcher.png"))
                    .expect("L’icône de fenêtre intégrée doit être une image valide")
                    .into_rgba8();
            let icon_texture = creation_context.egui_ctx.load_texture(
                "dofus-switcher-icon",
                egui::ColorImage::from_rgba_unmultiplied(
                    [icon_image.width() as usize, icon_image.height() as usize],
                    icon_image.as_raw(),
                ),
                egui::TextureOptions::LINEAR,
            );
            Ok(Box::new(WindowListApp::new(worker, icon_texture)))
        }),
    )
}

fn apply_dofus_style(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = egui::Color32::TRANSPARENT;
    visuals.window_fill = egui::Color32::from_rgb(27, 24, 35);
    visuals.extreme_bg_color = egui::Color32::from_rgb(16, 14, 21);
    visuals.text_edit_bg_color = Some(egui::Color32::from_rgb(23, 20, 30));
    visuals.override_text_color = Some(egui::Color32::from_rgb(239, 231, 216));
    visuals.weak_text_color = Some(egui::Color32::from_rgb(166, 157, 174));
    visuals.faint_bg_color = egui::Color32::from_rgb(32, 28, 41);
    visuals.selection.bg_fill = egui::Color32::from_rgb(80, 62, 42);
    visuals.selection.stroke = egui::Stroke::new(1.0, egui::Color32::from_rgb(224, 180, 99));
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(27, 24, 35);
    visuals.widgets.noninteractive.weak_bg_fill = egui::Color32::from_rgb(27, 24, 35);
    visuals.widgets.noninteractive.fg_stroke.color = egui::Color32::from_rgb(239, 231, 216);
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(40, 35, 50);
    visuals.widgets.inactive.weak_bg_fill = egui::Color32::from_rgb(40, 35, 50);
    visuals.widgets.inactive.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(61, 53, 72));
    visuals.widgets.inactive.fg_stroke.color = egui::Color32::from_rgb(222, 211, 194);
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(57, 48, 67);
    visuals.widgets.hovered.weak_bg_fill = egui::Color32::from_rgb(57, 48, 67);
    visuals.widgets.hovered.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(177, 137, 76));
    visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(255, 225, 166);
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(72, 56, 39);
    visuals.widgets.active.weak_bg_fill = egui::Color32::from_rgb(72, 56, 39);
    visuals.widgets.active.bg_stroke =
        egui::Stroke::new(1.0, egui::Color32::from_rgb(224, 180, 99));
    visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(255, 231, 190);
    for widget in [
        &mut visuals.widgets.noninteractive,
        &mut visuals.widgets.inactive,
        &mut visuals.widgets.hovered,
        &mut visuals.widgets.active,
        &mut visuals.widgets.open,
    ] {
        widget.corner_radius = egui::CornerRadius::same(6);
    }
    ctx.set_visuals(visuals);
    ctx.style_mut_of(egui::Theme::Dark, |style| {
        style.spacing.item_spacing = egui::vec2(8.0, 8.0);
        style.spacing.button_padding = egui::vec2(10.0, 6.0);
        style.spacing.text_edit_width = 180.0;
    });
}

pub(super) fn arrow_button(ui: &mut egui::Ui, up: bool, enabled: bool) -> bool {
    let response = ui.scope(|ui| {
        ui.spacing_mut().button_padding = egui::vec2(3.0, 2.0);
        ui.add_enabled(
            enabled,
            egui::Button::new(egui::RichText::new(if up { "↑" } else { "↓" }).font(
                egui::FontId::new(14.0, egui::FontFamily::Name("arrow_icons".into())),
            ))
            .min_size(egui::vec2(24.0, 24.0)),
        )
    });
    response.inner.clicked()
}

#[derive(Clone, Copy)]
pub(super) enum WindowControl {
    Minimize,
    Maximize,
    Close,
}

pub(super) fn window_control(ui: &mut egui::Ui, control: WindowControl) -> bool {
    let response = ui.add(
        egui::Button::new("")
            .frame(false)
            .min_size(egui::vec2(30.0, 28.0)),
    );
    if response.hovered() || response.is_pointer_button_down_on() {
        let fill = if matches!(control, WindowControl::Close) {
            egui::Color32::from_rgb(112, 48, 51)
        } else {
            egui::Color32::from_rgb(48, 42, 58)
        };
        ui.painter()
            .rect_filled(response.rect, egui::CornerRadius::same(5), fill);
    }

    let center = response.rect.center();
    let stroke = egui::Stroke::new(
        1.4,
        if matches!(control, WindowControl::Close) && response.hovered() {
            egui::Color32::from_rgb(255, 215, 206)
        } else {
            egui::Color32::from_rgb(211, 198, 177)
        },
    );
    match control {
        WindowControl::Minimize => {
            ui.painter().line_segment(
                [
                    egui::pos2(center.x - 4.0, center.y + 3.0),
                    egui::pos2(center.x + 4.0, center.y + 3.0),
                ],
                stroke,
            );
        }
        WindowControl::Maximize => {
            let rect = egui::Rect::from_center_size(center, egui::vec2(8.0, 8.0));
            ui.painter()
                .rect_stroke(rect, 1.0, stroke, egui::StrokeKind::Inside);
        }
        WindowControl::Close => {
            ui.painter().line_segment(
                [
                    egui::pos2(center.x - 4.0, center.y - 4.0),
                    egui::pos2(center.x + 4.0, center.y + 4.0),
                ],
                stroke,
            );
            ui.painter().line_segment(
                [
                    egui::pos2(center.x + 4.0, center.y - 4.0),
                    egui::pos2(center.x - 4.0, center.y + 4.0),
                ],
                stroke,
            );
        }
    }
    response.clicked()
}
