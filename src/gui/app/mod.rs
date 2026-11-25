mod calibration;
mod config_io;
mod initialization;
mod launch;
mod overlay_geometry;
mod overlay_persistence;
mod overlay_view;
mod view;
mod window_events;

pub(crate) use launch::run;

use std::collections::HashSet;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use eframe::egui;
use handy_keys::Hotkey;

use super::state::WindowListState;
use super::worker::WorkerHandle;
pub(super) struct WindowListApp {
    worker: WorkerHandle,
    app_icon: egui::TextureHandle,
    commands_tx: mpsc::Sender<super::worker::WindowCommand>,
    capture_rx: mpsc::Receiver<Hotkey>,
    state: WindowListState,
    registered_shortcuts: HashSet<u32>,
    overlay_tx: mpsc::Sender<OcrOverlayEvent>,
    overlay_rx: mpsc::Receiver<OcrOverlayEvent>,
    ocr_calibration: Option<OcrCalibration>,
    last_auto_sized_window_count: Option<usize>,
    last_saved_config: Option<crate::config::PersistedConfig>,
}

#[derive(Clone)]
pub(super) struct OcrCalibration {
    window: u32,
    geometry: crate::window::ClientGeometry,
    initial_placement: Option<OcrOverlayPlacement>,
    autosave: Arc<Mutex<OcrOverlayAutosave>>,
}

pub(super) struct OcrOverlayAutosave {
    previous_region: Option<crate::config::OcrRegion>,
    last_saved_region: Option<crate::config::OcrRegion>,
    last_saved_at: Instant,
}

#[derive(Clone, Copy)]
pub(super) struct OcrOverlayPlacement {
    position: [f32; 2],
    size: [f32; 2],
}

pub(super) enum OcrOverlayEvent {
    Save {
        rect: [f32; 4],
        pixels_per_point: f32,
    },
    Cancel(Option<crate::config::OcrRegion>),
    SaveFailed(String),
}
