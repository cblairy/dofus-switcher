use std::sync::mpsc;
use std::sync::{Arc, Mutex};

use eframe::egui;

use super::overlay_geometry::region_from_overlay;
use super::overlay_persistence::{autosave_ocr_region, cancel_ocr_overlay, restore_ocr_parent};
use crate::gui::app::{OcrOverlayAutosave, OcrOverlayEvent};

pub(super) fn render_ocr_overlay(
    ui: &mut egui::Ui,
    events_tx: &mpsc::Sender<OcrOverlayEvent>,
    geometry: crate::window::ClientGeometry,
    autosave: &Arc<Mutex<OcrOverlayAutosave>>,
) {
    let outer_rect = ui.input(|input| input.viewport().inner_rect.or(input.viewport().outer_rect));
    let scale = ui.ctx().native_pixels_per_point().unwrap_or(1.0);
    if let Some(rect) = outer_rect {
        autosave_ocr_region(
            geometry,
            [rect.min.x, rect.min.y, rect.max.x, rect.max.y],
            scale,
            autosave,
            events_tx,
        );
    }
    if ui.input(|input| input.key_pressed(egui::Key::Escape)) {
        cancel_ocr_overlay(ui.ctx(), autosave, events_tx);
        return;
    }
    if ui.input(|input| input.key_pressed(egui::Key::Enter)) {
        save_ocr_overlay(ui.ctx(), outer_rect, geometry, scale, events_tx);
        return;
    }
    egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(24, 21, 31, 170))
        .stroke(egui::Stroke::new(
            2.0,
            egui::Color32::from_rgb(223, 177, 98),
        ))
        .corner_radius(egui::CornerRadius::same(8))
        .inner_margin(egui::Margin::symmetric(6, 4))
        .show(ui, |ui| {
            let (handle, drag) =
                ui.allocate_exact_size(egui::vec2(ui.available_width(), 20.0), egui::Sense::drag());
            ui.painter().text(
                handle.left_center(),
                egui::Align2::LEFT_CENTER,
                "Glisse ↘ pour redimensionner · Entrée garder",
                egui::FontId::proportional(10.0),
                egui::Color32::from_rgb(237, 207, 153),
            );
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
            }
            let grip_rect = egui::Rect::from_min_size(
                ui.max_rect().right_bottom() + egui::vec2(4.0, 3.0) - egui::vec2(20.0, 20.0),
                egui::vec2(20.0, 20.0),
            );
            let grip = ui.interact(
                grip_rect,
                ui.id().with("ocr-overlay-resize"),
                egui::Sense::drag(),
            );
            if grip.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
            }
            if grip.drag_started() {
                ui.ctx()
                    .send_viewport_cmd(egui::ViewportCommand::BeginResize(
                        egui::ResizeDirection::SouthEast,
                    ));
            }
            let grip_color = if grip.hovered() || grip.dragged() {
                egui::Color32::WHITE
            } else {
                egui::Color32::from_rgb(255, 207, 105)
            };
            ui.painter().rect_filled(
                grip_rect.shrink(1.0),
                egui::CornerRadius::same(4),
                egui::Color32::from_rgba_unmultiplied(24, 21, 31, 235),
            );
            for offset in [7.0, 12.0, 17.0] {
                ui.painter().line_segment(
                    [
                        grip_rect.right_bottom() - egui::vec2(offset, 1.0),
                        grip_rect.right_bottom() - egui::vec2(1.0, offset),
                    ],
                    egui::Stroke::new(2.0, grip_color),
                );
            }
        });
}

pub(super) fn save_ocr_overlay(
    ctx: &egui::Context,
    outer_rect: Option<egui::Rect>,
    geometry: crate::window::ClientGeometry,
    scale: f32,
    events_tx: &mpsc::Sender<OcrOverlayEvent>,
) {
    if let Some(rect) = outer_rect {
        let bounds = [rect.min.x, rect.min.y, rect.max.x, rect.max.y];
        if let Some(region) = region_from_overlay(geometry, bounds, scale) {
            match crate::config::save_ocr_region(Some(region)) {
                Ok(()) => {
                    let _ = events_tx.send(OcrOverlayEvent::Save {
                        rect: bounds,
                        pixels_per_point: scale,
                    });
                    restore_ocr_parent(ctx);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Err(error) => {
                    let _ = events_tx.send(OcrOverlayEvent::SaveFailed(format!(
                        "Sauvegarde de la zone OCR impossible : {error:#}"
                    )));
                }
            }
        } else {
            let _ = events_tx.send(OcrOverlayEvent::SaveFailed(
                "Place la zone OCR à l’intérieur de la fenêtre Dofus.".to_owned(),
            ));
        }
    }
}
