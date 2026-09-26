use std::time::Duration;

use eframe::egui;
use x11rb::protocol::xproto::Window;

use super::state::WindowListState;
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
    commands_tx: std::sync::mpsc::Sender<Window>,
    state: WindowListState,
}

impl WindowListApp {
    fn new(worker: WorkerHandle) -> Self {
        let commands_tx = worker.commands_tx();
        Self {
            worker,
            commands_tx,
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
            }
        }
    }
}

impl eframe::App for WindowListApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.receive_worker_events();
        ui.heading("Detected Dofus windows");
        ui.separator();

        for error in self.state.errors.values() {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }

        if let Some(window) = self.state.focused_window {
            ui.label(format!(
                "Last focused Dofus window: {} (0x{window:08x})",
                self.state.character_label(window)
            ));
        } else {
            ui.label("No Dofus window detected.");
        }

        ui.separator();

        if self.state.windows.is_empty() {
            ui.label("No Dofus windows found.");
        } else {
            let windows = self.state.windows.clone();
            for (index, window) in windows.into_iter().enumerate() {
                ui.horizontal(|ui| {
                    let focused = self.state.focused_window == Some(window);
                    let marker = if focused { "● " } else { "  " };
                    if ui
                        .selectable_label(
                            self.state.selected_window == Some(window),
                            format!("{marker}Window {} · 0x{window:08x}", index + 1),
                        )
                        .clicked()
                    {
                        self.state.selected_window = Some(window);
                        if self.commands_tx.send(window).is_err() {
                            self.state.errors.insert(
                                WorkerKind::Activation,
                                "Window monitor is unavailable.".to_owned(),
                            );
                        }
                    }
                    ui.text_edit_singleline(self.state.character_names.entry(window).or_default());
                });
            }
        }

        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}
