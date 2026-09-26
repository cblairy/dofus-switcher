use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use eframe::egui;
use handy_keys::{Hotkey, KeyboardListener};

use super::state::{GlobalAction, WindowListState};
use super::worker::WorkerHandle;
use crate::runtime::{WorkerEvent, WorkerKind};

pub(crate) fn run() -> eframe::Result<()> {
    let worker = WorkerHandle::start();
    eframe::run_native(
        "Detected Dofus windows",
        eframe::NativeOptions::default(),
        Box::new(move |_creation_context| Ok(Box::new(WindowListApp::new(worker)))),
    )
}

struct WindowListApp {
    worker: WorkerHandle,
    commands_tx: mpsc::Sender<super::worker::WindowCommand>,
    capture_rx: mpsc::Receiver<Hotkey>,
    state: WindowListState,
}

impl WindowListApp {
    fn new(worker: WorkerHandle) -> Self {
        let commands_tx = worker.commands_tx();
        let (capture_tx, capture_rx) = mpsc::channel();
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
        Self {
            worker,
            commands_tx,
            capture_rx,
            state: WindowListState::default(),
        }
    }

    fn receive_worker_events(&mut self) {
        while let Ok(event) = self.worker.events_rx().try_recv() {
            match event {
                WorkerEvent::Windows {
                    windows,
                    focused_window,
                } => {
                    self.state.apply_windows(windows, focused_window);
                    self.state.errors.remove(&WorkerKind::Monitor);
                }
                WorkerEvent::Error { worker, message } => {
                    self.state.errors.insert(worker, message);
                }
                WorkerEvent::Recovered(worker) => {
                    self.state.errors.remove(&worker);
                }
                WorkerEvent::OcrText(text) => {
                    // Debug: show received OCR text and normalized form
                    let norm_text = crate::string_utils::normalize(&text);
                    eprintln!("[gui] OCR received: '{}' -> '{}'", text, norm_text);

                    // Build NameRegistry from UI-entered names and attempt to match
                    let mut reg = crate::name_registry::NameRegistry::new();
                    eprintln!("[gui] Candidate names:");
                    for (window, name) in &self.state.character_names {
                        eprintln!("  0x{window:08x} => '{}'", name);
                        reg.set_name(*window, name);
                    }

                    if let Some(target) = reg.find_best_match(&text) {
                        eprintln!("[gui] OCR matched -> activating 0x{target:08x}");
                        // activate the matched window
                        if self.commands_tx.send(super::worker::WindowCommand::Activate(target)).is_err() {
                            self.state.errors.insert(
                                WorkerKind::Activation,
                                "Le moniteur de fenêtres est indisponible.".to_owned(),
                            );
                        }
                    } else {
                        // no match — optional debug log
                        eprintln!("[gui] OCR no match for '{}'", text);
                    }
                }
            }
        }

        while let Ok(hotkey) = self.capture_rx.try_recv() {
            let shortcut = hotkey.to_string();
            if let Some(window) = self.state.capture_window {
                self.state.assign_shortcut(window, Some(shortcut.clone()));
                self.state.capture_window = None;
                self.state.errors.remove(&WorkerKind::Activation);
                let _ = self.commands_tx.send(super::worker::WindowCommand::AssignShortcut {
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
                        let _ = self.commands_tx.send(super::worker::WindowCommand::AssignGlobal {
                            action: crate::gui::state::GlobalAction::Next,
                            hotkey: Some(shortcut),
                        });
                    }
                    GlobalAction::Previous => {
                        self.state.previous_key = Some(shortcut.clone());
                        self.state.capture_global = None;
                        self.state.errors.remove(&WorkerKind::Activation);
                        let _ = self.commands_tx.send(super::worker::WindowCommand::AssignGlobal {
                            action: crate::gui::state::GlobalAction::Previous,
                            hotkey: Some(shortcut),
                        });
                    }
                }
            }
        }
    }
}

impl eframe::App for WindowListApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.receive_worker_events();
        ui.heading("Fenêtres Dofus détectées");
        ui.separator();

        for error in self.state.errors.values() {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }

        if let Some(window) = self.state.focused_window {
            ui.label(format!(
                "Dernière fenêtre Dofus focalisée : {} (0x{window:08x})",
                self.state.character_label(window)
            ));
        } else {
            ui.label("Aucune fenêtre Dofus détectée.");
        }

        ui.separator();

        // Global next/previous shortcut controls
        ui.horizontal(|ui| {
            ui.label("Raccourcis globaux :");

            // Previous
            if self.state.capture_global == Some(GlobalAction::Previous) {
                if ui.button("Appuyez sur une touche…").clicked() {
                    self.state.capture_global = None;
                }
            } else {
                let label = self
                    .state
                    .previous_key
                    .clone()
                    .unwrap_or_else(|| "Précédent".to_owned());
                if ui.button(format!("Précédent : {}", label)).clicked() {
                    self.state.capture_global = Some(GlobalAction::Previous);
                }
            }

            // Next
            if self.state.capture_global == Some(GlobalAction::Next) {
                if ui.button("Appuyez sur une touche…").clicked() {
                    self.state.capture_global = None;
                }
            } else {
                let label = self
                    .state
                    .next_key
                    .clone()
                    .unwrap_or_else(|| "Suivant".to_owned());
                if ui.button(format!("Suivant : {}", label)).clicked() {
                    self.state.capture_global = Some(GlobalAction::Next);
                }
            }
        });

        if self.state.windows.is_empty() {
            ui.label("Aucune fenêtre Dofus trouvée.");
        } else {
            let windows = self.state.windows.clone();
            for (index, window) in windows.into_iter().enumerate() {
                ui.horizontal(|ui| {
                    let focused = self.state.focused_window == Some(window);
                    let marker = if focused { "● " } else { "  " };
                    if ui
                        .selectable_label(
                            self.state.selected_window == Some(window),
                            format!("{marker}Fenêtre {} · 0x{window:08x}", index + 1),
                        )
                        .clicked()
                    {
                        self.state.selected_window = Some(window);
                        if self
                            .commands_tx
                            .send(super::worker::WindowCommand::Activate(window))
                            .is_err()
                        {
                            self.state.errors.insert(
                                WorkerKind::Activation,
                                "Le moniteur de fenêtres est indisponible.".to_owned(),
                            );
                        }
                    }
                    ui.add(
                        egui::TextEdit::singleline(
                            self.state.character_names.entry(window).or_default(),
                        )
                        .hint_text("Nom du personnage"),
                    );

                    if self.state.capture_window == Some(window) {
                        if ui.button("Appuyez sur une touche…").clicked() {
                            self.state.capture_window = None;
                        }
                    } else {
                        let button_label = self
                            .state
                            .shortcut_keys
                            .get(&window)
                            .cloned()
                            .unwrap_or_else(|| "Touche".to_owned());
                        if ui.button(button_label).clicked() {
                            self.state.capture_window = Some(window);
                        }
                    }
                });
            }
        }

        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}
