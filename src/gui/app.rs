use std::collections::HashSet;
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use handy_keys::{Hotkey, KeyboardListener};

use super::state::{GlobalAction, WindowListState};
use super::worker::WorkerHandle;
use crate::runtime::{WorkerEvent, WorkerKind};

pub(crate) fn run() -> eframe::Result<()> {
    let worker = WorkerHandle::start();
    let icon = image::load_from_memory(include_bytes!("../../assets/icons/dofus-switcher.png"))
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
                    "../../assets/fonts/NotoSansSymbols-Arrows.ttf"
                ))
                .into(),
            );
            fonts.families.insert(
                egui::FontFamily::Name("arrow_icons".into()),
                vec!["arrow_icons".to_owned()],
            );
            creation_context.egui_ctx.set_fonts(fonts);
            let icon_image =
                image::load_from_memory(include_bytes!("../../assets/icons/dofus-switcher.png"))
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

fn arrow_button(ui: &mut egui::Ui, up: bool, enabled: bool) -> bool {
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
enum WindowControl {
    Minimize,
    Maximize,
    Close,
}

fn window_control(ui: &mut egui::Ui, control: WindowControl) -> bool {
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

struct WindowListApp {
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
}

#[derive(Clone)]
struct OcrCalibration {
    window: u32,
    geometry: crate::window::ClientGeometry,
    initial_placement: Option<OcrOverlayPlacement>,
    autosave: Arc<Mutex<OcrOverlayAutosave>>,
}

struct OcrOverlayAutosave {
    previous_region: Option<crate::config::OcrRegion>,
    last_saved_region: Option<crate::config::OcrRegion>,
    last_saved_at: Instant,
}

#[derive(Clone, Copy)]
struct OcrOverlayPlacement {
    position: [f32; 2],
    size: [f32; 2],
}

enum OcrOverlayEvent {
    Save {
        rect: [f32; 4],
        pixels_per_point: f32,
    },
    Cancel(Option<crate::config::OcrRegion>),
    SaveFailed(String),
}

impl WindowListApp {
    fn new(worker: WorkerHandle, app_icon: egui::TextureHandle) -> Self {
        let commands_tx = worker.commands_tx();
        let (capture_tx, capture_rx) = mpsc::channel();
        let (overlay_tx, overlay_rx) = mpsc::channel();
        let listener = KeyboardListener::new();

        if let Ok(listener) = listener {
            let tx = capture_tx;
            thread::spawn(move || {
                while let Ok(event) = listener.recv() {
                    if event.is_key_down {
                        let Ok(hotkey) = event.as_hotkey() else {
                            continue;
                        };
                        let _ = tx.send(hotkey);
                    }
                }
            });
        }

        let mut state = WindowListState::default();

        // chargement de la config persistée
        if let Ok(config) = crate::config::load() {
            if config.next_key.is_some() {
                state.next_key = config.next_key.clone();
            }
            if config.previous_key.is_some() {
                state.previous_key = config.previous_key.clone();
            }
            state.restrict_prev_next_to_dofus = config.restrict_prev_next_to_dofus;
            state.ocr_region = config.ocr_region;
            // The old config stored characters in discovery order and initiative
            // separately. Reorder that roster once, then use names as identities.
            let mut characters = config.characters;
            let mut ordered_characters = Vec::with_capacity(characters.len());
            for index in config.window_order {
                if index < characters.len() {
                    ordered_characters.push(characters[index].clone());
                }
            }
            for character in characters.drain(..) {
                if !ordered_characters
                    .iter()
                    .any(|entry| entry.name == character.name)
                {
                    ordered_characters.push(character);
                }
            }
            state.persisted_characters = discard_incremental_name_fragments(ordered_characters);

            if let Some(hotkey) = state.next_key.clone() {
                let _ = commands_tx.send(super::worker::WindowCommand::AssignGlobal {
                    action: GlobalAction::Next,
                    hotkey: Some(hotkey),
                });
            }
            if let Some(hotkey) = state.previous_key.clone() {
                let _ = commands_tx.send(super::worker::WindowCommand::AssignGlobal {
                    action: GlobalAction::Previous,
                    hotkey: Some(hotkey),
                });
            }

            let _ = commands_tx.send(super::worker::WindowCommand::SetRestrictPrevNextToDofus(
                state.restrict_prev_next_to_dofus,
            ));
        }

        worker.set_ocr_region(state.ocr_region);

        Self {
            worker,
            app_icon,
            commands_tx,
            capture_rx,
            state,
            registered_shortcuts: HashSet::new(),
            overlay_tx,
            overlay_rx,
            ocr_calibration: None,
            last_auto_sized_window_count: None,
        }
    }

    fn begin_ocr_calibration(&mut self, ctx: &egui::Context) -> bool {
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

    fn receive_overlay_events(&mut self) {
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

    fn sync_window_shortcuts(&mut self) {
        for &window in &self.state.windows {
            let Some(hotkey) = self.state.shortcut_keys.get(&window).cloned() else {
                continue;
            };

            if self.registered_shortcuts.contains(&window) {
                continue;
            }

            if self
                .commands_tx
                .send(super::worker::WindowCommand::AssignShortcut {
                    window,
                    hotkey: Some(hotkey),
                })
                .is_err()
            {
                self.state.errors.insert(
                    WorkerKind::Activation,
                    "Le moniteur de fenêtres est indisponible.".to_owned(),
                );
                return;
            }

            self.registered_shortcuts.insert(window);
        }

        self.registered_shortcuts
            .retain(|window| self.state.windows.contains(window));
    }

    fn refresh_window_assignments(&mut self) {
        self.registered_shortcuts.clear();
        for &window in &self.state.detected_windows {
            let hotkey = self.state.shortcut_keys.get(&window).cloned();
            if hotkey.is_some() {
                self.registered_shortcuts.insert(window);
            }
            let _ = self
                .commands_tx
                .send(super::worker::WindowCommand::AssignShortcut { window, hotkey });
        }
        let _ = self
            .commands_tx
            .send(super::worker::WindowCommand::SetWindowOrder(
                self.state.windows.clone(),
            ));
    }

    fn receive_worker_events(&mut self) {
        while let Ok(event) = self.worker.events_rx().try_recv() {
            match event {
                WorkerEvent::Windows {
                    windows,
                    focused_window,
                } => {
                    self.state.apply_windows(windows, focused_window);
                    let _ = self
                        .commands_tx
                        .send(super::worker::WindowCommand::SetWindowOrder(
                            self.state.windows.clone(),
                        ));
                    self.sync_window_shortcuts();
                    self.state.errors.remove(&WorkerKind::Monitor);
                }
                WorkerEvent::Error { worker, message } => {
                    self.state.errors.insert(worker, message);
                }
                WorkerEvent::Recovered(worker) => {
                    self.state.errors.remove(&worker);
                }
                WorkerEvent::OcrText(text) => {
                    self.state.ocr_text = text.clone();
                    let mut reg = crate::name_registry::NameRegistry::new();
                    for (window, name) in &self.state.character_names {
                        reg.set_name(*window, name);
                    }

                    if let Some(target) = reg.find_best_match(&text) {
                        self.state.ocr_target = Some(target);
                        if self.state.ocr_candidate == Some(target) {
                            self.state.ocr_candidate_hits =
                                self.state.ocr_candidate_hits.saturating_add(1);
                        } else {
                            self.state.ocr_candidate = Some(target);
                            self.state.ocr_candidate_hits = 1;
                            self.state.ocr_candidate_switched = false;
                        }
                        if self.state.ocr_candidate_hits >= 2 && !self.state.ocr_candidate_switched
                        {
                            if self
                                .commands_tx
                                .send(super::worker::WindowCommand::Activate(target))
                                .is_err()
                            {
                                self.state.errors.insert(
                                    WorkerKind::Activation,
                                    "Le moniteur de fenêtres est indisponible.".to_owned(),
                                );
                            }
                            self.state.ocr_candidate_switched = true;
                        }
                    } else {
                        self.state.ocr_target = None;
                        self.state.ocr_candidate = None;
                        self.state.ocr_candidate_hits = 0;
                        self.state.ocr_candidate_switched = false;
                    }
                }
            }
        }

        while let Ok(hotkey) = self.capture_rx.try_recv() {
            let shortcut = hotkey.to_string();
            if let Some(window) = self.state.capture_window {
                self.state.assign_shortcut(window, Some(shortcut.clone()));
                self.registered_shortcuts.insert(window);
                self.state.capture_window = None;
                self.state.errors.remove(&WorkerKind::Activation);
                let _ = self
                    .commands_tx
                    .send(super::worker::WindowCommand::AssignShortcut {
                        window,
                        hotkey: Some(shortcut),
                    });
                continue;
            }

            if let Some(action) = self.state.capture_global {
                match action {
                    GlobalAction::Next => {
                        self.state.next_key = Some(shortcut.clone());
                        self.state.capture_global = None;
                        self.state.errors.remove(&WorkerKind::Activation);
                        let _ = self
                            .commands_tx
                            .send(super::worker::WindowCommand::AssignGlobal {
                                action: crate::gui::state::GlobalAction::Next,
                                hotkey: Some(shortcut),
                            });
                    }
                    GlobalAction::Previous => {
                        self.state.previous_key = Some(shortcut.clone());
                        self.state.capture_global = None;
                        self.state.errors.remove(&WorkerKind::Activation);
                        let _ = self
                            .commands_tx
                            .send(super::worker::WindowCommand::AssignGlobal {
                                action: crate::gui::state::GlobalAction::Previous,
                                hotkey: Some(shortcut),
                            });
                    }
                }
            }
        }
    }

    fn persist_config(&self) {
        let config = crate::config::PersistedConfig {
            version: 4,
            characters: self.state.persisted_characters.clone(),
            ocr_region: self.state.ocr_region,
            window_order: Vec::new(),
            next_key: self.state.next_key.clone(),
            previous_key: self.state.previous_key.clone(),
            restrict_prev_next_to_dofus: self.state.restrict_prev_next_to_dofus,
        };

        if let Err(error) = crate::config::save(&config) {
            eprintln!("[config] Echec de la sauvegarde : {error:#}");
        }
    }
}

fn discard_incremental_name_fragments(
    characters: Vec<crate::config::CharacterConfig>,
) -> Vec<crate::config::CharacterConfig> {
    let mut fragments = HashSet::new();
    for complete_name in characters
        .iter()
        .filter(|character| character.shortcut.is_none())
    {
        let complete = complete_name.name.to_lowercase();
        let prefixes: Vec<_> = characters
            .iter()
            .filter(|character| {
                character.shortcut.is_none()
                    && character.name.chars().count() >= 2
                    && character.name.chars().count() < complete_name.name.chars().count()
                    && complete.starts_with(&character.name.to_lowercase())
            })
            .collect();
        // Older builds saved every intermediate value while the user typed.
        // Only remove a clear chain of at least three such prefixes; retain
        // shorter legitimate character names and anything with a shortcut.
        if prefixes.len() >= 3 {
            fragments.extend(prefixes.into_iter().map(|character| character.name.clone()));
        }
    }
    characters
        .into_iter()
        .filter(|character| !fragments.contains(&character.name))
        .collect()
}

fn region_from_overlay(
    geometry: crate::window::ClientGeometry,
    rect: [f32; 4],
    pixels_per_point: f32,
) -> Option<crate::config::OcrRegion> {
    let scale = pixels_per_point.max(0.1);
    let left = (rect[0] * scale - f32::from(geometry.x)).clamp(0.0, f32::from(geometry.width));
    let top = (rect[1] * scale - f32::from(geometry.y)).clamp(0.0, f32::from(geometry.height));
    let right = (rect[2] * scale - f32::from(geometry.x)).clamp(0.0, f32::from(geometry.width));
    let bottom = (rect[3] * scale - f32::from(geometry.y)).clamp(0.0, f32::from(geometry.height));
    if right - left < 8.0 || bottom - top < 8.0 {
        return None;
    }
    Some(crate::config::OcrRegion {
        x: left / f32::from(geometry.width),
        y: top / f32::from(geometry.height),
        width: (right - left) / f32::from(geometry.width),
        height: (bottom - top) / f32::from(geometry.height),
    })
}

fn render_ocr_overlay(
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

fn restore_ocr_parent(ctx: &egui::Context) {
    let parent = ctx.input(|input| input.viewport().parent.unwrap_or(egui::ViewportId::ROOT));
    ctx.send_viewport_cmd_to(parent, egui::ViewportCommand::Minimized(false));
}

fn save_ocr_overlay(
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

impl eframe::App for WindowListApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.receive_worker_events();
        self.receive_overlay_events();
        let window_count = self.state.windows.len();
        let mut content_height = 0.0;
        let available = ui.available_size();
        let inner_size = egui::vec2((available.x - 2.0).max(0.0), (available.y - 2.0).max(0.0));

        egui::Frame::new()
            .fill(egui::Color32::from_rgb(24, 21, 31))
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_rgb(68, 58, 77)))
            .corner_radius(egui::CornerRadius::same(11))
            .inner_margin(egui::Margin::ZERO)
            .show(ui, |ui| {
                ui.set_min_size(inner_size);

                ui.horizontal(|ui| {
                    let width = ui.available_width();
                    let spacing = ui.spacing().item_spacing.x;
                    let brand_width = 150.0;
                    let left_width = (width * 0.5 - brand_width * 0.5).max(0.0);
                    let control_width = 3.0 * 30.0 + 3.0 * spacing;
                    let right_width =
                        (width - left_width - brand_width - control_width - 2.0 * spacing).max(0.0);

                    let (_, left_drag) = ui.allocate_exact_size(
                        egui::vec2(left_width, 30.0),
                        egui::Sense::click_and_drag(),
                    );
                    let (brand_rect, brand_drag) = ui.allocate_exact_size(
                        egui::vec2(brand_width, 30.0),
                        egui::Sense::click_and_drag(),
                    );
                    let icon_rect = egui::Rect::from_center_size(
                        egui::pos2(brand_rect.center().x - 48.0, brand_rect.center().y),
                        egui::vec2(18.0, 18.0),
                    );
                    ui.painter().image(
                        self.app_icon.id(),
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
                    let (_, right_drag) = ui.allocate_exact_size(
                        egui::vec2(right_width, 30.0),
                        egui::Sense::click_and_drag(),
                    );
                    if left_drag.drag_started()
                        || brand_drag.drag_started()
                        || right_drag.drag_started()
                    {
                        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                    }

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
                });

                egui::Frame::new()
                    .outer_margin(egui::Margin {
                        left: 12,
                        right: 0,
                        top: 0,
                        bottom: 0,
                    })
                    .show(ui, |ui| {
                        let scroll_output = egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                // Keep a small gutter between cards and the window border
                                // without moving the scrollbar away from the right edge.
                                ui.set_max_width((ui.available_width() - 8.0).max(0.0));
                                ui.add_space(4.0);
                                ui.horizontal(|ui| {
                                    let (emblem_rect, _) = ui.allocate_exact_size(
                                        egui::vec2(34.0, 34.0),
                                        egui::Sense::hover(),
                                    );
                                    let center = emblem_rect.center();
                                    ui.painter().circle_filled(
                                        center,
                                        15.0,
                                        egui::Color32::from_rgb(48, 38, 48),
                                    );
                                    ui.painter().circle_stroke(
                                        center,
                                        14.0,
                                        egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_rgb(206, 160, 83),
                                        ),
                                    );
                                    let gem = [
                                        egui::pos2(center.x, center.y - 8.0),
                                        egui::pos2(center.x + 5.0, center.y),
                                        egui::pos2(center.x, center.y + 8.0),
                                        egui::pos2(center.x - 5.0, center.y),
                                    ];
                                    ui.painter().add(egui::Shape::convex_polygon(
                                        gem.to_vec(),
                                        egui::Color32::from_rgb(100, 185, 151),
                                        egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_rgb(224, 190, 124),
                                        ),
                                    ));
                                    ui.vertical(|ui| {
                                        ui.label(
                                            egui::RichText::new("ORDRE D’INITIATIVE")
                                                .size(18.0)
                                                .strong()
                                                .color(egui::Color32::from_rgb(237, 207, 153)),
                                        );
                                        ui.label(
                                            egui::RichText::new(
                                                "GESTION DES COMPTES · SWITCH RAPIDE",
                                            )
                                            .size(10.0)
                                            .color(egui::Color32::from_rgb(166, 157, 174)),
                                        );
                                    });
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            let count = self.state.windows.len();
                                            let count_text = if count == 1 {
                                                "1 COMPTE".to_owned()
                                            } else {
                                                format!("{count} COMPTES")
                                            };
                                            egui::Frame::new()
                                                .fill(egui::Color32::from_rgb(37, 46, 43))
                                                .stroke(egui::Stroke::new(
                                                    1.0,
                                                    egui::Color32::from_rgb(67, 107, 87),
                                                ))
                                                .corner_radius(egui::CornerRadius::same(12))
                                                .inner_margin(egui::Margin::symmetric(10, 5))
                                                .show(ui, |ui| {
                                                    ui.label(
                                                        egui::RichText::new(count_text)
                                                            .size(10.0)
                                                            .strong()
                                                            .color(egui::Color32::from_rgb(
                                                                144, 215, 176,
                                                            )),
                                                    );
                                                });
                                        },
                                    );
                                });

                                ui.add_space(10.0);
                                ui.separator();

                                for error in self.state.errors.values() {
                                    egui::Frame::new()
                                        .fill(egui::Color32::from_rgb(61, 31, 37))
                                        .corner_radius(egui::CornerRadius::same(6))
                                        .inner_margin(egui::Margin::symmetric(10, 6))
                                        .show(ui, |ui| {
                                            ui.colored_label(
                                                egui::Color32::from_rgb(250, 166, 155),
                                                error,
                                            );
                                        });
                                }

                                egui::Frame::new()
                                    .fill(egui::Color32::from_rgb(27, 24, 35))
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(52, 46, 62),
                                    ))
                                    .corner_radius(egui::CornerRadius::same(8))
                                    .inner_margin(egui::Margin::symmetric(10, 8))
                                    .show(ui, |ui| {
                                        ui.set_min_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    egui::RichText::new("CYCLE DE COMPTES")
                                                        .size(10.0)
                                                        .strong()
                                                        .color(egui::Color32::from_rgb(
                                                            202, 167, 104,
                                                        )),
                                                );
                                                ui.label(
                                                    egui::RichText::new("Raccourcis globaux")
                                                        .size(11.0)
                                                        .color(egui::Color32::from_rgb(
                                                            166, 157, 174,
                                                        )),
                                                );
                                            });
                                            ui.separator();
                                            ui.add_space((ui.available_width() - 220.0).max(0.0));
                                            if self.state.capture_global
                                                == Some(GlobalAction::Previous)
                                            {
                                                if ui.button("Appuyez sur une touche…").clicked()
                                                {
                                                    self.state.capture_global = None;
                                                }
                                            } else {
                                                let key = self
                                                    .state
                                                    .previous_key
                                                    .as_deref()
                                                    .unwrap_or("—");
                                                if ui.button(format!("Précédent   {key}")).clicked()
                                                {
                                                    self.state.capture_global =
                                                        Some(GlobalAction::Previous);
                                                }
                                            }
                                            if self.state.capture_global == Some(GlobalAction::Next)
                                            {
                                                if ui.button("Appuyez sur une touche…").clicked()
                                                {
                                                    self.state.capture_global = None;
                                                }
                                            } else {
                                                let key =
                                                    self.state.next_key.as_deref().unwrap_or("—");
                                                if ui.button(format!("Suivant   {key}")).clicked() {
                                                    self.state.capture_global =
                                                        Some(GlobalAction::Next);
                                                }
                                            }
                                        });
                                    });

                                ui.add_space(8.0);
                                egui::Frame::new()
                                    .fill(egui::Color32::from_rgb(27, 24, 35))
                                    .stroke(egui::Stroke::new(
                                        1.0,
                                        egui::Color32::from_rgb(52, 46, 62),
                                    ))
                                    .corner_radius(egui::CornerRadius::same(8))
                                    .inner_margin(egui::Margin::symmetric(6, 4))
                                    .show(ui, |ui| {
                                        ui.set_min_width(ui.available_width());
                                        ui.horizontal(|ui| {
                                            ui.vertical(|ui| {
                                                ui.label(
                                                    egui::RichText::new("LECTURE DU TOUR · OCR")
                                                        .size(10.0)
                                                        .strong()
                                                        .color(egui::Color32::from_rgb(202, 167, 104)),
                                                );
                                                let status = if let Some(window) = self.state.ocr_target {
                                                    format!("Détecté : {}", self.state.character_label(window))
                                                } else if !self.state.ocr_text.trim().is_empty() {
                                                    format!("Texte lu : {}", self.state.ocr_text.trim())
                                                } else if self.state.ocr_region.is_some() {
                                                    "Zone active · attente du nom de tour".to_owned()
                                                } else {
                                                    "Place la zone sur la vignette du début de tour.".to_owned()
                                                };
                                                ui.label(
                                                    egui::RichText::new(status)
                                                        .size(10.0)
                                                        .color(egui::Color32::from_rgb(166, 157, 174)),
                                                );
                                            });
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    if self.state.ocr_region.is_some()
                                                        && ui.button("Désactiver").clicked()
                                                    {
                                                        self.state.ocr_region = None;
                                                        self.state.ocr_text.clear();
                                                        self.state.ocr_target = None;
                                                        self.state.ocr_candidate = None;
                                                        self.state.ocr_candidate_hits = 0;
                                                        self.state.ocr_candidate_switched = false;
                                                        self.worker.set_ocr_region(None);
                                                    }
                                                    if ui.button(if self.state.ocr_region.is_some() {
                                                        "Repositionner"
                                                    } else {
                                                        "Placer la zone"
                                                    }).clicked() {
                                                        if self.begin_ocr_calibration(ui.ctx()) {
                                                            ui.ctx().send_viewport_cmd(
                                                                egui::ViewportCommand::Minimized(true),
                                                            );
                                                        }
                                                    }
                                                    if let Some(window) = self.state.ocr_target {
                                                        if ui.button("Basculer").clicked()
                                                            && self.commands_tx.send(
                                                                super::worker::WindowCommand::Activate(window),
                                                            ).is_err()
                                                        {
                                                            self.state.errors.insert(
                                                                WorkerKind::Activation,
                                                                "Le moniteur de fenêtres est indisponible.".to_owned(),
                                                            );
                                                        }
                                                    }
                                                },
                                            );
                                        });
                                    });

                                if self.state.windows.is_empty() {
                                    ui.add_space(12.0);
                                    egui::Frame::new()
                                        .fill(egui::Color32::from_rgb(27, 24, 35))
                                        .stroke(egui::Stroke::new(
                                            1.0,
                                            egui::Color32::from_rgb(52, 46, 62),
                                        ))
                                        .corner_radius(egui::CornerRadius::same(8))
                                        .inner_margin(egui::Margin::symmetric(16, 18))
                                        .show(ui, |ui| {
                                            ui.vertical_centered(|ui| {
                                                ui.label(
                                                    egui::RichText::new(
                                                        "Aucun compte Dofus détecté",
                                                    )
                                                    .size(15.0)
                                                    .strong()
                                                    .color(egui::Color32::from_rgb(223, 211, 192)),
                                                );
                                                ui.label(
                            egui::RichText::new(
                                "Lance tes fenêtres de jeu pour les ajouter à l’ordre de tour.",
                            )
                            .size(11.0)
                            .color(egui::Color32::from_rgb(166, 157, 174)),
                        );
                                            });
                                        });
                                } else {
                                    ui.add_space(12.0);
                                    ui.horizontal(|ui| {
                                        ui.label(
                                            egui::RichText::new("ORDRE DE TOUR")
                                                .size(10.0)
                                                .strong()
                                                .color(egui::Color32::from_rgb(202, 167, 104)),
                                        );
                                        ui.separator();
                                        let unassigned = self
                                            .state
                                            .windows
                                            .iter()
                                            .filter(|window| {
                                                self.state
                                                    .character_names
                                                    .get(window)
                                                    .is_none_or(|name| name.trim().is_empty())
                                            })
                                            .count();
                                        if unassigned > 0 {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{unassigned} À ASSOCIER"
                                                ))
                                                .size(10.0)
                                                .strong()
                                                .color(egui::Color32::from_rgb(213, 168, 94)),
                                            );
                                        }
                                        if let Some(window) = self.state.focused_window {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "ACTIF · {}",
                                                    self.state.character_label(window)
                                                ))
                                                .size(10.0)
                                                .color(egui::Color32::from_rgb(144, 215, 176)),
                                            );
                                        } else {
                                            ui.label(
                                                egui::RichText::new("AUCUN COMPTE FOCALISÉ")
                                                    .size(10.0)
                                                    .color(egui::Color32::from_rgb(166, 157, 174)),
                                            );
                                        }
                                    });
                                    let windows = self.state.windows.clone();
                                    let saved_characters = self.state.persisted_characters.clone();
                                    if let Some(window) = self.state.association_window {
                                        let mut selected_character = None;
                                        let mut create_name = None;
                                        egui::Frame::new()
                                            .fill(egui::Color32::from_rgb(34, 29, 40))
                                            .stroke(egui::Stroke::new(
                                                1.0,
                                                egui::Color32::from_rgb(111, 85, 55),
                                            ))
                                            .corner_radius(egui::CornerRadius::same(8))
                                            .inner_margin(egui::Margin::symmetric(10, 8))
                                            .show(ui, |ui| {
                                                ui.set_min_width(ui.available_width());
                                                ui.horizontal(|ui| {
                                                    ui.label(
                                                        egui::RichText::new("ASSOCIER UN PERSONNAGE")
                                                            .size(10.0)
                                                            .strong()
                                                            .color(egui::Color32::from_rgb(213, 177, 112)),
                                                    );
                                                            let position = self
                                                                .state
                                                                .windows
                                                                .iter()
                                                                .position(|candidate| *candidate == window)
                                                                .unwrap_or(0)
                                                                + 1;
                                                            ui.label(
                                                                egui::RichText::new(format!(
                                                                    "pour le compte {position:02}"
                                                                ))
                                                                .size(10.0)
                                                                .color(egui::Color32::from_rgb(166, 157, 174)),
                                                            );
                                                    ui.with_layout(
                                                        egui::Layout::right_to_left(egui::Align::Center),
                                                        |ui| {
                                                            if ui.small_button("Annuler").clicked() {
                                                                self.state.association_window = None;
                                                                self.state.association_query.clear();
                                                            }
                                                        },
                                                    );
                                                });
                                                ui.label(
                                                    egui::RichText::new(
                                                        "Choisis un personnage enregistré ou crée un nouveau nom.",
                                                    )
                                                    .size(10.0)
                                                    .color(egui::Color32::from_rgb(166, 157, 174)),
                                                );
                                                ui.add(
                                                    egui::TextEdit::singleline(
                                                        &mut self.state.association_query,
                                                    )
                                                    .desired_width(ui.available_width())
                                                    .hint_text("Rechercher ou saisir un nom"),
                                                );

                                                let query = self.state.association_query.trim().to_lowercase();
                                                let matches: Vec<_> = saved_characters
                                                    .iter()
                                                    .filter(|character| {
                                                        query.is_empty()
                                                            || character.name.to_lowercase().contains(&query)
                                                    })
                                                    .collect();
                                                if matches.is_empty() {
                                                    ui.label(
                                                        egui::RichText::new("Aucun personnage enregistré ne correspond.")
                                                            .size(10.0)
                                                            .color(egui::Color32::from_rgb(166, 157, 174)),
                                                    );
                                                } else {
                                                    egui::ScrollArea::vertical()
                                                        .max_height(96.0)
                                                        .show(ui, |ui| {
                                                            ui.horizontal_wrapped(|ui| {
                                                                for character in matches {
                                                                    if ui.button(&character.name).clicked() {
                                                                        selected_character = Some(character.name.clone());
                                                                    }
                                                                }
                                                            });
                                                        });
                                                }

                                                let query = self.state.association_query.trim();
                                                if !query.is_empty()
                                                    && !saved_characters.iter().any(|character| {
                                                        character.name.eq_ignore_ascii_case(query)
                                                    })
                                                {
                                                    if ui.button(format!("Créer « {query} » et associer")).clicked() {
                                                        create_name = Some(query.to_owned());
                                                    }
                                                }
                                            });

                                        if let Some(name) = selected_character {
                                            self.state.assign_character(window, name);
                                            self.state.association_window = None;
                                            self.state.association_query.clear();
                                            self.refresh_window_assignments();
                                        } else if let Some(name) = create_name {
                                            self.state.create_character(window, name);
                                            self.state.association_window = None;
                                            self.state.association_query.clear();
                                            self.refresh_window_assignments();
                                        }
                                        ui.add_space(6.0);
                                    }
                                    for (index, window) in windows.into_iter().enumerate() {
                                        let focused = self.state.focused_window == Some(window);
                                        let row_fill = if focused {
                                            egui::Color32::from_rgb(35, 47, 43)
                                        } else {
                                            egui::Color32::from_rgb(27, 24, 35)
                                        };
                                        let row_stroke = if focused {
                                            egui::Color32::from_rgb(83, 129, 99)
                                        } else {
                                            egui::Color32::from_rgb(49, 43, 58)
                                        };
                                        egui::Frame::new()
                                            .fill(row_fill)
                                            .stroke(egui::Stroke::new(1.0, row_stroke))
                                            .corner_radius(egui::CornerRadius::same(7))
                                            .inner_margin(egui::Margin::symmetric(9, 5))
                                            .show(ui, |ui| {
                                                ui.set_min_width(ui.available_width());
                                                ui.horizontal(|ui| {
                                                    if ui
                                                        .selectable_label(
                                                            self.state.selected_window
                                                                == Some(window),
                                                            egui::RichText::new(format!(
                                                                "{:02}",
                                                                index + 1
                                                            ))
                                                            .size(11.0)
                                                            .strong(),
                                                        )
                                                        .clicked()
                                                    {
                                                        self.state.selected_window = Some(window);
                                                        if self
                                                    .commands_tx
                                                    .send(super::worker::WindowCommand::Activate(
                                                        window,
                                                    ))
                                                    .is_err()
                                                {
                                                    self.state.errors.insert(
                                                        WorkerKind::Activation,
                                                        "Le moniteur de fenêtres est indisponible."
                                                            .to_owned(),
                                                    );
                                                }
                                                    }
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "0x{window:08x}"
                                                        ))
                                                        .size(10.0)
                                                        .color(egui::Color32::from_rgb(
                                                            132, 124, 143,
                                                        )),
                                                    );
                                                    let character_name = self
                                                        .state
                                                        .character_names
                                                        .get(&window)
                                                        .filter(|name| !name.trim().is_empty())
                                                        .map(String::as_str)
                                                        .unwrap_or("Aucun personnage lié");
                                                    ui.add(
                                                        egui::Label::new(egui::RichText::new(character_name)
                                                            .color(if character_name == "Aucun personnage lié" {
                                                                egui::Color32::from_rgb(153, 145, 161)
                                                            } else {
                                                                egui::Color32::from_rgb(239, 231, 216)
                                                            }))
                                                        .truncate(),
                                                    );
                                                    if ui.button(if character_name == "Aucun personnage lié" {
                                                        "Associer…"
                                                    } else {
                                                        "Changer"
                                                    }).clicked() {
                                                        self.state.association_window = Some(window);
                                                        self.state.association_query.clear();
                                                        self.state.selected_window = Some(window);
                                                    }

                                                    if self.state.capture_window == Some(window) {
                                                        if ui
                                                            .button("Appuyez sur une touche…")
                                                            .clicked()
                                                        {
                                                            self.state.capture_window = None;
                                                        }
                                                    } else {
                                                        let button_label = self
                                                            .state
                                                            .shortcut_keys
                                                            .get(&window)
                                                            .cloned()
                                                            .unwrap_or_else(|| {
                                                                "Raccourci".to_owned()
                                                            });
                                                        if ui.button(button_label).clicked() {
                                                            self.state.capture_window =
                                                                Some(window);
                                                        }
                                                    }
                                                    if arrow_button(ui, true, index > 0) {
                                                        self.state.move_window(window, -1);
                                                        let _ = self.commands_tx.send(
                                                    super::worker::WindowCommand::SetWindowOrder(
                                                        self.state.windows.clone(),
                                                    ),
                                                );
                                                    }
                                                    if arrow_button(
                                                        ui,
                                                        false,
                                                        index + 1 < self.state.windows.len(),
                                                    ) {
                                                        self.state.move_window(window, 1);
                                                        let _ = self.commands_tx.send(
                                                    super::worker::WindowCommand::SetWindowOrder(
                                                        self.state.windows.clone(),
                                                    ),
                                                );
                                                    }
                                                });
                                            });
                                    }
                                }
                            });
                        content_height = scroll_output.content_size.y;
                    });

                let window_rect = ui.max_rect();
                let grip_size = egui::vec2(20.0, 20.0);
                let grip_rect =
                    egui::Rect::from_min_size(window_rect.right_bottom() - grip_size, grip_size);
                let grip = ui.interact(
                    grip_rect,
                    ui.id().with("window-resize-grip"),
                    egui::Sense::drag(),
                );
                if grip.drag_started() {
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::BeginResize(
                            egui::ResizeDirection::SouthEast,
                        ));
                }
                let grip_color = if grip.hovered() {
                    egui::Color32::from_rgb(194, 157, 101)
                } else {
                    egui::Color32::from_rgb(87, 77, 96)
                };
                for offset in [5.0, 9.0, 13.0] {
                    ui.painter().line_segment(
                        [
                            grip_rect.right_bottom() - egui::vec2(offset, 1.0),
                            grip_rect.right_bottom() - egui::vec2(1.0, offset),
                        ],
                        egui::Stroke::new(1.0, grip_color),
                    );
                }
                self.state.sync_character_profiles();
                self.persist_config();
            });
        if self.last_auto_sized_window_count != Some(window_count) {
            self.last_auto_sized_window_count = Some(window_count);
            let current_width = ui
                .ctx()
                .input(|input| input.viewport().inner_rect.map(|rect| rect.width()))
                .unwrap_or(600.0);
            let monitor_height = ui
                .ctx()
                .input(|input| input.viewport().monitor_size.map(|size| size.y))
                .unwrap_or(900.0)
                .max(260.0);
            let desired_height = (content_height + 62.0)
                .max(220.0)
                .min(monitor_height - 40.0);
            ui.ctx()
                .send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                    current_width,
                    desired_height,
                )));
        }
        if let Some(calibration) = self.ocr_calibration.clone() {
            let viewport_id = egui::ViewportId::from_hash_of("ocr-calibration-zone");
            let tx = self.overlay_tx.clone();
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
                    render_ocr_overlay(overlay_ui, &tx, geometry, &autosave)
                });
            if let Some(calibration) = &mut self.ocr_calibration {
                calibration.initial_placement = None;
            }
        }
        if !ui
            .ctx()
            .input(|input| input.viewport().minimized.unwrap_or(false))
        {
            ui.ctx().request_repaint_after(Duration::from_millis(100));
        }
    }
}

fn autosave_ocr_region(
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

fn cancel_ocr_overlay(
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
