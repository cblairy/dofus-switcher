use std::sync::{Arc, Mutex};
use std::time::Instant;

use eframe::egui;

use crate::runtime::WorkerKind;

use super::overlay_geometry::region_from_overlay;
use super::{
    OcrCalibration, OcrOverlayAutosave, OcrOverlayEvent, OcrOverlayPlacement, WindowListApp,
};

impl WindowListApp {
    pub(super) fn begin_ocr_calibration(&mut self, ctx: &egui::Context) -> bool {
        let Some(window) = self
            .state
            .selected_window
            .or(self.state.focused_window)
            .or_else(|| self.state.windows.first().copied())
        else {
            self.state.errors.insert(
                WorkerKind::Ocr,
                "Sélectionne d’abord une fenêtre Dofus pour placer la zone OCR.".to_owned(),
            );
            return false;
        };
        match crate::window::client_geometry(window) {
            Ok(geometry) => {
                let scale = ctx.native_pixels_per_point().unwrap_or(1.0).max(0.1);
                let initial_placement = if let Some(region) = self.state.ocr_region {
                    Some(OcrOverlayPlacement {
                        position: [
                            (f32::from(geometry.x)
                                + region.x.clamp(0.0, 1.0) * f32::from(geometry.width))
                                / scale,
                            (f32::from(geometry.y)
                                + region.y.clamp(0.0, 1.0) * f32::from(geometry.height))
                                / scale,
                        ],
                        size: [
                            (region.width.clamp(0.001, 1.0) * f32::from(geometry.width) / scale)
                                .max(92.0),
                            (region.height.clamp(0.001, 1.0) * f32::from(geometry.height) / scale)
                                .max(34.0),
                        ],
                    })
                } else {
                    Some(OcrOverlayPlacement {
                        position: [
                            (f32::from(geometry.x) + f32::from(geometry.width) * 0.5
                                - 160.0 * scale)
                                / scale,
                            (f32::from(geometry.y) + f32::from(geometry.height) * 0.5
                                - 52.0 * scale)
                                / scale,
                        ],
                        size: [320.0, 106.0],
                    })
                };
                let initial_region = initial_placement.and_then(|placement| {
                    region_from_overlay(
                        geometry,
                        [
                            placement.position[0],
                            placement.position[1],
                            placement.position[0] + placement.size[0],
                            placement.position[1] + placement.size[1],
                        ],
                        scale,
                    )
                });
                self.ocr_calibration = Some(OcrCalibration {
                    window,
                    geometry,
                    initial_placement,
                    autosave: Arc::new(Mutex::new(OcrOverlayAutosave {
                        previous_region: self.state.ocr_region,
                        last_saved_region: initial_region,
                        last_saved_at: Instant::now(),
                    })),
                });
                self.state.errors.remove(&WorkerKind::Ocr);
                true
            }
            Err(error) => {
                self.state.errors.insert(
                    WorkerKind::Ocr,
                    format!("Impossible de placer l’overlay OCR : {error:#}"),
                );
                false
            }
        }
    }

    pub(super) fn receive_overlay_events(&mut self) {
        while let Ok(event) = self.overlay_rx.try_recv() {
            match event {
                OcrOverlayEvent::Save {
                    rect,
                    pixels_per_point,
                } => {
                    if let Some(calibration) = self.ocr_calibration.take() {
                        if !self.state.detected_windows.contains(&calibration.window) {
                            self.state.errors.insert(
                                WorkerKind::Ocr,
                                "La fenêtre Dofus de référence a été fermée pendant le placement."
                                    .to_owned(),
                            );
                            continue;
                        }
                        let geometry = crate::window::client_geometry(calibration.window)
                            .unwrap_or(calibration.geometry);
                        if let Some(region) = region_from_overlay(geometry, rect, pixels_per_point)
                        {
                            self.state.ocr_region = Some(region);
                            self.worker.set_ocr_region(Some(region));
                            self.state.errors.remove(&WorkerKind::Ocr);
                            self.persist_config();
                        } else {
                            self.state.errors.insert(
                                WorkerKind::Ocr,
                                "Place la zone OCR à l’intérieur de la fenêtre Dofus.".to_owned(),
                            );
                        }
                    }
                }
                OcrOverlayEvent::Cancel(previous_region) => {
                    self.ocr_calibration = None;
                    self.state.ocr_region = previous_region;
                    self.state.ocr_text.clear();
                    self.state.ocr_target = None;
                    self.state.ocr_candidate = None;
                    self.state.ocr_candidate_hits = 0;
                    self.state.ocr_candidate_switched = false;
                    self.worker.set_ocr_region(previous_region);
                    self.persist_config();
                }
                OcrOverlayEvent::SaveFailed(message) => {
                    self.state.errors.insert(WorkerKind::Ocr, message);
                }
            }
        }
    }
}
