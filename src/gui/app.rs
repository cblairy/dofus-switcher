use std::time::Duration;

use eframe::egui;

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
    state: WindowListState,
}

impl WindowListApp {
    fn new(worker: WorkerHandle) -> Self {
        Self {
            worker,
            state: WindowListState::default(),
        }
    }

    fn receive_worker_events(&mut self) {
        while let Ok(event) = self.worker.events_rx().try_recv() {
            match event {
                WorkerEvent::Windows(windows) => {
                    self.state.windows = windows;
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

        if self.state.windows.is_empty() {
            ui.label("No Dofus windows found.");
        } else {
            for (index, window) in self.state.windows.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("Window {}", index + 1));
                    ui.label(format!("ID: 0x{window:08x}"));
                });
            }
        }

        ui.ctx().request_repaint_after(Duration::from_millis(100));
    }
}
