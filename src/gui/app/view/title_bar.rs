use eframe::egui;

use crate::gui::app::WindowListApp;
use crate::gui::app::launch::{WindowControl, window_control};

pub(super) fn show(app: &WindowListApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        let width = ui.available_width();
        let spacing = ui.spacing().item_spacing.x;
        let brand_width = 150.0;
        let left_width = (width * 0.5 - brand_width * 0.5).max(0.0);
        let control_width = 3.0 * 30.0 + 3.0 * spacing;
        let right_width =
            (width - left_width - brand_width - control_width - 2.0 * spacing).max(0.0);

        let (_, left_drag) =
            ui.allocate_exact_size(egui::vec2(left_width, 30.0), egui::Sense::click_and_drag());
        let (brand_rect, brand_drag) =
            ui.allocate_exact_size(egui::vec2(brand_width, 30.0), egui::Sense::click_and_drag());
        paint_brand(app, ui, brand_rect);
        let (_, right_drag) =
            ui.allocate_exact_size(egui::vec2(right_width, 30.0), egui::Sense::click_and_drag());
        if left_drag.drag_started() || brand_drag.drag_started() || right_drag.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }
        handle_window_controls(ui);
    });
}

fn paint_brand(app: &WindowListApp, ui: &egui::Ui, brand_rect: egui::Rect) {
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(brand_rect.center().x - 48.0, brand_rect.center().y),
        egui::vec2(18.0, 18.0),
    );
    ui.painter().image(
        app.app_icon.id(),
        icon_rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    ui.painter().text(
        egui::pos2(icon_rect.right() + 8.0, brand_rect.center().y),
        egui::Align2::LEFT_CENTER,
        "Dofus Switcher",
        egui::FontId::proportional(13.0),
        egui::Color32::from_rgb(166, 157, 174),
    );
}

fn handle_window_controls(ui: &mut egui::Ui) {
    if window_control(ui, WindowControl::Minimize) {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
    }
    if window_control(ui, WindowControl::Maximize) {
        let maximized = ui
            .ctx()
            .input(|input| input.viewport().maximized.unwrap_or(false));
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
    }
    if window_control(ui, WindowControl::Close) {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
    }
}
