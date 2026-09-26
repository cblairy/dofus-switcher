use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::autoswitch;
use crate::runtime::{WorkerEvent, WorkerKind};
use crate::window;
use anyhow::anyhow;

pub(super) struct WorkerHandle {
    events_rx: Receiver<WorkerEvent>,
    commands_tx: Sender<u32>,
    monitor_stop_tx: Sender<()>,
    ocr_stop_tx: Sender<()>,
    threads: Vec<JoinHandle<()>>,
}

impl WorkerHandle {
    pub(super) fn start() -> Self {
        let (events_tx, events_rx) = mpsc::channel();
        let (commands_tx, commands_rx) = mpsc::channel();
        let (monitor_stop_tx, monitor_stop_rx) = mpsc::channel();
        let (ocr_stop_tx, ocr_stop_rx) = mpsc::channel();
        let monitor_events_tx = events_tx.clone();

        let monitor =
            thread::spawn(move || window_monitor(monitor_events_tx, commands_rx, monitor_stop_rx));
        let ocr = thread::spawn(move || {
            let result = (|| {
                let (conn, _) = x11rb::connect(None)
                    .map_err(|error| anyhow!("Failed to connect OCR worker to X11: {error}"))?;
                autoswitch::auto_switch(&conn, ocr_stop_rx, events_tx.clone())
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
            threads: vec![monitor, ocr],
        }
    }

    pub(super) fn events_rx(&self) -> &Receiver<WorkerEvent> {
        &self.events_rx
    }

    pub(super) fn commands_tx(&self) -> Sender<u32> {
        self.commands_tx.clone()
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

fn window_monitor(
    updates_tx: Sender<WorkerEvent>,
    commands_rx: Receiver<u32>,
    stop_rx: Receiver<()>,
) {
    let (conn, _) = match x11rb::connect(None) {
        Ok(connection) => connection,
        Err(error) => {
            let _ = updates_tx.send(WorkerEvent::Error {
                worker: WorkerKind::Monitor,
                message: format!("Failed to connect window monitor to X11: {error}"),
            });
            return;
        }
    };

    let mut tracker = window::DofusWindowTracker::new();
    let mut last_scan_error = None;
    loop {
        match stop_rx.recv_timeout(Duration::from_millis(300)) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        if let Ok(window) = commands_rx.try_recv()
            && let Err(error) = window::activation::activate(&conn, window)
        {
            let _ = updates_tx.send(WorkerEvent::Error {
                worker: WorkerKind::Activation,
                message: format!("Failed to activate window 0x{window:08x}: {error:#}"),
            });
        }

        match tracker.refresh(&conn) {
            Ok(focused_window) => {
                if last_scan_error.take().is_some()
                    && updates_tx
                        .send(WorkerEvent::Recovered(WorkerKind::Monitor))
                        .is_err()
                {
                    break;
                }
                if updates_tx
                    .send(WorkerEvent::Windows {
                        windows: tracker.windows().to_vec(),
                        focused_window,
                    })
                    .is_err()
                {
                    break;
                }
            }
            Err(error) => {
                let message = format!("Window monitor scan failed: {error:#}");
                if last_scan_error.as_deref() != Some(message.as_str()) {
                    if updates_tx
                        .send(WorkerEvent::Error {
                            worker: WorkerKind::Monitor,
                            message: message.clone(),
                        })
                        .is_err()
                    {
                        break;
                    }
                    last_scan_error = Some(message);
                }
            }
        }
    }
}
