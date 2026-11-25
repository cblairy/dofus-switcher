use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui;

use super::overlay_geometry::region_from_overlay;
use crate::gui::app::{OcrOverlayAutosave, OcrOverlayEvent};

pub(super) fn restore_ocr_parent(ctx: &egui::Context) {
    let parent = ctx.input(|input| input.viewport().parent.unwrap_or(egui::ViewportId::ROOT));
    ctx.send_viewport_cmd_to(parent, egui::ViewportCommand::Minimized(false));
}

pub(super) fn autosave_ocr_region(
    geometry: crate::window::ClientGeometry,
    rect: [f32; 4],
    scale: f32,
    autosave: &Arc<Mutex<OcrOverlayAutosave>>,
    events_tx: &mpsc::Sender<OcrOverlayEvent>,
) {
    let Some(region) = region_from_overlay(geometry, rect, scale) else {
        return;
    };
    let Ok(mut autosave) = autosave.lock() else {
        return;
    };
    if autosave
        .last_saved_region
        .is_some_and(|saved| regions_are_close(saved, region))
        || autosave.last_saved_at.elapsed() < Duration::from_millis(250)
    {
        return;
    }
    autosave.last_saved_at = Instant::now();
    match crate::config::save_ocr_region(Some(region)) {
        Ok(()) => autosave.last_saved_region = Some(region),
        Err(error) => {
            let _ = events_tx.send(OcrOverlayEvent::SaveFailed(format!(
                "Sauvegarde automatique de la zone OCR impossible : {error:#}"
            )));
        }
    }
}

fn regions_are_close(a: crate::config::OcrRegion, b: crate::config::OcrRegion) -> bool {
    let tolerance = 0.001;
    (a.x - b.x).abs() < tolerance
        && (a.y - b.y).abs() < tolerance
        && (a.width - b.width).abs() < tolerance
        && (a.height - b.height).abs() < tolerance
}

pub(super) fn cancel_ocr_overlay(
    ctx: &egui::Context,
    autosave: &Arc<Mutex<OcrOverlayAutosave>>,
    events_tx: &mpsc::Sender<OcrOverlayEvent>,
) {
    let previous_region = autosave.lock().ok().and_then(|state| state.previous_region);
    if let Err(error) = crate::config::save_ocr_region(previous_region) {
        let _ = events_tx.send(OcrOverlayEvent::SaveFailed(format!(
            "Restauration de la zone OCR impossible : {error:#}"
        )));
    }
    let _ = events_tx.send(OcrOverlayEvent::Cancel(previous_region));
    restore_ocr_parent(ctx);
    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
}
