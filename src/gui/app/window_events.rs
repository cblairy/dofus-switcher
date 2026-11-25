use crate::gui::state::GlobalAction;
use crate::runtime::{WorkerEvent, WorkerKind};

use super::WindowListApp;

impl WindowListApp {
    pub(super) fn sync_window_shortcuts(&mut self) {
        for &window in &self.state.windows {
            let Some(hotkey) = self.state.shortcut_keys.get(&window).cloned() else {
                continue;
            };

            if self.registered_shortcuts.contains(&window) {
                continue;
            }

            if self
                .commands_tx
                .send(crate::gui::worker::WindowCommand::AssignShortcut {
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

    pub(super) fn refresh_window_assignments(&mut self) {
        self.registered_shortcuts.clear();
        for &window in &self.state.detected_windows {
            let hotkey = self.state.shortcut_keys.get(&window).cloned();
            if hotkey.is_some() {
                self.registered_shortcuts.insert(window);
            }
            let _ = self
                .commands_tx
                .send(crate::gui::worker::WindowCommand::AssignShortcut { window, hotkey });
        }
        let _ = self
            .commands_tx
            .send(crate::gui::worker::WindowCommand::SetWindowOrder(
                self.state.windows.clone(),
            ));
    }

    pub(super) fn receive_worker_events(&mut self) {
        while let Ok(event) = self.worker.events_rx().try_recv() {
            match event {
                WorkerEvent::Windows {
                    windows,
                    focused_window,
                } => {
                    self.state.apply_windows(windows, focused_window);
                    let _ =
                        self.commands_tx
                            .send(crate::gui::worker::WindowCommand::SetWindowOrder(
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
                                .send(crate::gui::worker::WindowCommand::Activate(target))
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
                    .send(crate::gui::worker::WindowCommand::AssignShortcut {
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
                        let _ = self.commands_tx.send(
                            crate::gui::worker::WindowCommand::AssignGlobal {
                                action: crate::gui::state::GlobalAction::Next,
                                hotkey: Some(shortcut),
                            },
                        );
                    }
                    GlobalAction::Previous => {
                        self.state.previous_key = Some(shortcut.clone());
                        self.state.capture_global = None;
                        self.state.errors.remove(&WorkerKind::Activation);
                        let _ = self.commands_tx.send(
                            crate::gui::worker::WindowCommand::AssignGlobal {
                                action: crate::gui::state::GlobalAction::Previous,
                                hotkey: Some(shortcut),
                            },
                        );
                    }
                }
            }
        }
    }
}
