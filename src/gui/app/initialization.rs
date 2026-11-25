use std::collections::HashSet;
use std::sync::mpsc;
use std::thread;

use eframe::egui;
use handy_keys::KeyboardListener;

use crate::gui::state::{GlobalAction, WindowListState};
use crate::gui::worker::WorkerHandle;

use super::WindowListApp;
use super::config_io::discard_incremental_name_fragments;
use super::help_images::HelpImages;

impl WindowListApp {
    pub(super) fn new(
        worker: WorkerHandle,
        app_icon: egui::TextureHandle,
        help_images: HelpImages,
    ) -> Self {
        let commands_tx = worker.commands_tx();
        let (capture_tx, capture_rx) = mpsc::channel();
        let (overlay_tx, overlay_rx) = mpsc::channel();
        let listener = KeyboardListener::new();
        let listener_error = match listener {
            Ok(listener) => {
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
                None
            }
            Err(error) => Some(format!(
                "Écoute clavier indisponible : impossible d’accéder à /dev/input ({error:#}). La détection et le changement de fenêtres restent actifs ; vérifiez les permissions du périphérique clavier."
            )),
        };

        let mut state = WindowListState::default();
        if let Some(message) = listener_error {
            state
                .errors
                .insert(crate::runtime::WorkerKind::HotkeyCapture, message);
        }

        // chargement de la config persistée
        match crate::config::load() {
            Ok(config) => {
                if config.next_key.is_some() {
                    state.next_key = config.next_key.clone();
                }
                if config.previous_key.is_some() {
                    state.previous_key = config.previous_key.clone();
                }
                state.restrict_prev_next_to_dofus = config.restrict_prev_next_to_dofus;
                state.ocr_region = config.ocr_region;
                state.saved_window_assignments = config
                    .window_assignments
                    .into_iter()
                    .map(|assignment| (assignment.window, assignment.character))
                    .collect();
                let configured_initiative_order = config.initiative_order;
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
                state.initiative_order = if configured_initiative_order.is_empty() {
                    state
                        .persisted_characters
                        .iter()
                        .map(|character| character.name.clone())
                        .collect()
                } else {
                    configured_initiative_order
                };
                state.auto_order_restore_pending = !state.saved_window_assignments.is_empty();

                if let Some(hotkey) = state.next_key.clone() {
                    let _ = commands_tx.send(crate::gui::worker::WindowCommand::AssignGlobal {
                        action: GlobalAction::Next,
                        hotkey: Some(hotkey),
                    });
                }
                if let Some(hotkey) = state.previous_key.clone() {
                    let _ = commands_tx.send(crate::gui::worker::WindowCommand::AssignGlobal {
                        action: GlobalAction::Previous,
                        hotkey: Some(hotkey),
                    });
                }

                let _ = commands_tx.send(
                    crate::gui::worker::WindowCommand::SetRestrictPrevNextToDofus(
                        state.restrict_prev_next_to_dofus,
                    ),
                );
            }
            Err(error) => {
                state.config_error = Some(format!("Configuration non chargée : {error:#}"));
            }
        }

        worker.set_ocr_region(state.ocr_region);

        let mut app = Self {
            worker,
            app_icon,
            help_images,
            commands_tx,
            capture_rx,
            state,
            registered_shortcuts: HashSet::new(),
            overlay_tx,
            overlay_rx,
            ocr_calibration: None,
            last_auto_sized_window_count: None,
            last_saved_config: None,
            help_open: false,
        };
        app.state.sync_character_profiles();
        app.last_saved_config = Some(app.persisted_config());
        app
    }
}
