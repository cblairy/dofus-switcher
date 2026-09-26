use std::collections::HashMap;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::autoswitch;
use crate::runtime::{WorkerEvent, WorkerKind};
use crate::window;
use anyhow::anyhow;
use handy_keys::{Hotkey, HotkeyId, HotkeyManager, HotkeyState};
use x11rb::protocol::xproto::Window;

pub(super) enum WindowCommand {
    Activate(Window),
    AssignShortcut { window: Window, hotkey: Option<String> },
}

pub(super) struct WorkerHandle {
    events_rx: Receiver<WorkerEvent>,
    commands_tx: Sender<WindowCommand>,
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

    pub(super) fn commands_tx(&self) -> Sender<WindowCommand> {
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
    commands_rx: Receiver<WindowCommand>,
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

    let manager = match HotkeyManager::new() {
        Ok(manager) => manager,
        Err(error) => {
            let _ = updates_tx.send(WorkerEvent::Error {
                worker: WorkerKind::Activation,
                message: format!("Failed to initialize global hotkey manager: {error:#}"),
            });
            return;
        }
    };

    let mut window_shortcuts: HashMap<Window, Hotkey> = HashMap::new();
    let mut hotkey_windows: HashMap<HotkeyId, Window> = HashMap::new();
    let mut tracker = window::DofusWindowTracker::new();
    let mut last_scan_error = None;

    loop {
        match stop_rx.recv_timeout(Duration::from_millis(300)) {
            Ok(()) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }

        while let Ok(command) = commands_rx.try_recv() {
            match command {
                WindowCommand::Activate(window) => {
                    if let Err(error) = window::activation::activate(&conn, window) {
                        let _ = updates_tx.send(WorkerEvent::Error {
                            worker: WorkerKind::Activation,
                            message: format!("Failed to activate window 0x{window:08x}: {error:#}"),
                        });
                    }
                }
                WindowCommand::AssignShortcut { window, hotkey } => {
                    let Some(hotkey) = hotkey.filter(|value| !value.trim().is_empty()) else {
                        unregister_window_shortcut(&manager, &mut window_shortcuts, &mut hotkey_windows, window);
                        continue;
                    };

                    let parsed = match hotkey.trim().parse::<Hotkey>() {
                        Ok(parsed) => parsed,
                        Err(error) => {
                            let _ = updates_tx.send(WorkerEvent::Error {
                                worker: WorkerKind::Activation,
                                message: format!(
                                    "Invalid shortcut '{hotkey}' for window 0x{window:08x}: {error:#}"
                                ),
                            });
                            continue;
                        }
                    };

                    unregister_window_shortcut(&manager, &mut window_shortcuts, &mut hotkey_windows, window);

                    let duplicate_window = window_shortcuts
                        .iter()
                        .find_map(|(mapped_window, mapped_hotkey)| (*mapped_hotkey == parsed).then_some(*mapped_window));
                    if let Some(duplicate_window) = duplicate_window {
                        unregister_window_shortcut(&manager, &mut window_shortcuts, &mut hotkey_windows, duplicate_window);
                    }

                    match manager.register(parsed) {
                        Ok(id) => {
                            window_shortcuts.insert(window, parsed);
                            hotkey_windows.insert(id, window);
                        }
                        Err(error) => {
                            let _ = updates_tx.send(WorkerEvent::Error {
                                worker: WorkerKind::Activation,
                                message: format!(
                                    "Failed to register shortcut '{hotkey}' for window 0x{window:08x}: {error:#}"
                                ),
                            });
                        }
                    }
                }
            }
        }

        while let Some(event) = manager.try_recv() {
            let Some(target_window) = hotkey_windows.get(&event.id).copied() else {
                continue;
            };
            if event.state == HotkeyState::Pressed
                && let Err(error) = window::activation::activate(&conn, target_window)
            {
                let _ = updates_tx.send(WorkerEvent::Error {
                    worker: WorkerKind::Activation,
                    message: format!("Failed to activate shortcut window 0x{target_window:08x}: {error:#}"),
                });
            }
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

fn unregister_window_shortcut(
    manager: &HotkeyManager,
    window_shortcuts: &mut HashMap<Window, Hotkey>,
    hotkey_windows: &mut HashMap<HotkeyId, Window>,
    window: Window,
) {
    if let Some(hotkey) = window_shortcuts.remove(&window) {
        let hotkey_id = hotkey_windows
            .iter()
            .find_map(|(id, mapped_window)| (*mapped_window == window && window_shortcuts.get(&window) == Some(&hotkey)).then_some(*id));
        if let Some(id) = hotkey_id {
            let _ = manager.unregister(id);
            hotkey_windows.remove(&id);
        }
    }
}
