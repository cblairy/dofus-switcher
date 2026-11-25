mod account_list;
mod account_row;
mod alerts;
mod association;
mod global_shortcuts;
mod ocr_status;
mod overview;
mod resize_grip;
mod title_bar;

use std::time::Duration;

use eframe::egui;

use super::overlay_view::render_ocr_overlay;
use crate::gui::app::WindowListApp;

impl eframe::App for WindowListApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.receive_worker_events();
        self.receive_overlay_events();
        let count = self.state.windows.len();
        let content_height = draw_main_window(self, ui);
        resize_after_account_change(self, ui, count, content_height);
        show_calibration_viewport(self, ui);
        request_repaint_when_visible(ui);
    }
}

fn draw_main_window(app: &mut WindowListApp, ui: &mut egui::Ui) -> f32 {
    let available = ui.available_size();
    let min_size = egui::vec2((available.x - 2.0).max(0.0), (available.y - 2.0).max(0.0));
    let mut content_height = 0.0;
    egui::Frame::new()
        .fill(egui::Color32::from_rgb(24, 21, 31))
        .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 58, 77)))
        .corner_radius(egui::CornerRadius::same(11))
        .inner_margin(egui::Margin::ZERO)
        .show(ui, |ui| {
            ui.set_min_size(min_size);
            title_bar::show(app, ui);
            let output = show_content(app, ui);
            content_height = output;
            resize_grip::show(ui);
            app.state.sync_character_profiles();
            app.persist_config();
        });
    content_height
}

fn show_content(app: &mut WindowListApp, ui: &mut egui::Ui) -> f32 {
    egui::Frame::new()
        .outer_margin(egui::Margin {
            left: 12,
            right: 0,
            top: 0,
            bottom: 0,
        })
        .show(ui, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.set_max_width((ui.available_width() - 8.0).max(0.0));
                    overview::show(app, ui);
                    alerts::show(app, ui);
                    global_shortcuts::show(app, ui);
                    ocr_status::show(app, ui);
                    account_list::show(app, ui);
                })
                .content_size
                .y
        })
        .inner
}

fn resize_after_account_change(
    app: &mut WindowListApp,
    ui: &egui::Ui,
    count: usize,
    content_height: f32,
) {
    if app.last_auto_sized_window_count == Some(count) {
        return;
    }
    app.last_auto_sized_window_count = Some(count);
    let width = ui
        .ctx()
        .input(|input| input.viewport().inner_rect.map(|rect| rect.width()))
        .unwrap_or(600.0);
    let monitor_height = ui
        .ctx()
        .input(|input| input.viewport().monitor_size.map(|size| size.y))
        .unwrap_or(900.0)
        .max(260.0);
    let height = (content_height + 262.0)
        .max(220.0)
        .min(monitor_height - 40.0);
    ui.ctx()
        .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width, height)));
}

fn show_calibration_viewport(app: &mut WindowListApp, ui: &mut egui::Ui) {
    let Some(calibration) = app.ocr_calibration.clone() else {
        return;
    };
    let viewport_id = egui::ViewportId::from_hash_of("ocr-calibration-zone");
    let tx = app.overlay_tx.clone();
    let geometry = calibration.geometry;
    let autosave = calibration.autosave.clone();
    let mut builder = egui::ViewportBuilder::default()
        .with_title("Dofus Switcher OCR Zone")
        .with_min_inner_size([92.0, 34.0])
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_resizable(true)
        .with_taskbar(false);
    if let Some(placement) = calibration.initial_placement {
        builder = builder
            .with_position(placement.position)
            .with_inner_size(placement.size);
    }
    ui.ctx()
        .show_viewport_deferred(viewport_id, builder, move |overlay_ui, _class| {
            render_ocr_overlay(overlay_ui, &tx, geometry, &autosave);
        });
    if let Some(calibration) = &mut app.ocr_calibration {
        calibration.initial_placement = None;
    }
}

fn request_repaint_when_visible(ui: &egui::Ui) {
    if !ui
        .ctx()
        .input(|input| input.viewport().minimized.unwrap_or(false))
    {
        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}
