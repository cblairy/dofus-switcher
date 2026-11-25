use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};

use crate::autoswitch;
use crate::gui::worker::WindowCommand;
use crate::runtime::{WorkerEvent, WorkerKind};
use anyhow::anyhow;

pub(crate) struct WorkerHandle {
    events_rx: Receiver<WorkerEvent>,
    commands_tx: Sender<WindowCommand>,
    monitor_stop_tx: Sender<()>,
    ocr_stop_tx: Sender<()>,
    ocr_settings_tx: Sender<Option<crate::config::OcrRegion>>,
    threads: Vec<JoinHandle<()>>,
}

impl WorkerHandle {
    pub(crate) fn start() -> Self {
        let (events_tx, events_rx) = mpsc::channel();
        let (commands_tx, commands_rx) = mpsc::channel();
        let (monitor_stop_tx, monitor_stop_rx) = mpsc::channel();
        let (ocr_stop_tx, ocr_stop_rx) = mpsc::channel();
        let (ocr_settings_tx, ocr_settings_rx) = mpsc::channel();
        let monitor_events_tx = events_tx.clone();

        let monitor = thread::spawn(move || {
            super::monitor::window_monitor(monitor_events_tx, commands_rx, monitor_stop_rx)
        });
        let ocr = thread::spawn(move || {
            let result = (|| {
                let (conn, _) = x11rb::connect(None)
                    .map_err(|error| anyhow!("Failed to connect OCR worker to X11: {error}"))?;
                autoswitch::auto_switch(&conn, ocr_stop_rx, ocr_settings_rx, events_tx.clone())
            })();
            if let Err(error) = result {
                let _ = events_tx.send(WorkerEvent::Error {
                    worker: WorkerKind::Ocr,
                    message: format!("OCR worker stopped: {error:#}"),
                });
            }
        });

        Self {
            events_rx,
            commands_tx,
            monitor_stop_tx,
            ocr_stop_tx,
            ocr_settings_tx,
            threads: vec![monitor, ocr],
        }
    }

    pub(crate) fn events_rx(&self) -> &Receiver<WorkerEvent> {
        &self.events_rx
    }

    pub(crate) fn commands_tx(&self) -> Sender<WindowCommand> {
        self.commands_tx.clone()
    }

    pub(crate) fn set_ocr_region(&self, region: Option<crate::config::OcrRegion>) {
        let _ = self.ocr_settings_tx.send(region);
    }
}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        let _ = self.monitor_stop_tx.send(());
        let _ = self.ocr_stop_tx.send(());
        for worker in self.threads.drain(..) {
            let _ = worker.join();
        }
    }
}
