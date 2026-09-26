use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::Duration;

use anyhow::anyhow;
use eframe::egui;
use x11rb::protocol::xproto::Window;

use crate::window;

pub fn run() -> eframe::Result<()> {
    thread::spawn(|| {
        if let Ok((conn, _)) = x11rb::connect(None) {
            crate::autoswitch::auto_switch(&conn);
        }
    });

    let (updates_tx, updates_rx) = mpsc::channel();
    thread::spawn(move || {
        let (conn, _) = match x11rb::connect(None) {
            Ok(connection) => connection,
            Err(error) => {
                let _ = updates_tx.send(Err(anyhow!("Failed to connect to X11: {error}")));
                return;
            }
        };

        loop {
            let update = window::find_dofus_windows(&conn);
            if updates_tx.send(update).is_err() {
                break;
            }
            thread::sleep(Duration::from_secs(1));
        }
    });

    eframe::run_native(
        "Dofus Switcher",
        eframe::NativeOptions::default(),
        Box::new(move |_creation_context| {
            Ok(Box::new(WindowListApp {
                updates_rx,
                windows: Vec::new(),
                error: None,
            }))
        }),
    )
}

struct WindowListApp {
    updates_rx: Receiver<anyhow::Result<Vec<Window>>>,
    windows: Vec<Window>,
    error: Option<String>,
}

impl eframe::App for WindowListApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        while let Ok(update) = self.updates_rx.try_recv() {
            match update {
                Ok(windows) => {
                    self.windows = windows;
                    self.error = None;
                }
                Err(error) => self.error = Some(format!("Window scan failed: {error:#}")),
            }
        }

        ui.heading("Detected Dofus windows");
        ui.separator();

        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        } else if self.windows.is_empty() {
            ui.label("No Dofus windows detected.");
        } else {
            for (index, window) in self.windows.iter().enumerate() {
                ui.horizontal(|ui| {
                    ui.label(format!("Window {}", index + 1));
                    ui.label(format!("ID: 0x{window:08x}"));
                });
            }
        }

        ui.ctx().request_repaint_after(Duration::from_millis(250));
    }
}
