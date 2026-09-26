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

use crate::gui::state::GlobalAction;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HotkeyAction {
    Window(Window),
    Next,
    Previous,
}

pub(super) enum WindowCommand {
    Activate(Window),
    AssignShortcut { window: Window, hotkey: Option<String> },
    AssignGlobal { action: GlobalAction, hotkey: Option<String> },
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

fn send_worker_error(updates_tx: &Sender<WorkerEvent>, worker: WorkerKind, message: String) {
    let _ = updates_tx.send(WorkerEvent::Error { worker, message });
}

// Message helpers (centralise les chaînes localisées)
fn msg_activate_window(window: Window, error: &impl std::fmt::Display) -> String {
    format!("Échec d'activation de la fenêtre 0x{window:08x} : {error}")
}

fn msg_register_shortcut_window(hotkey: &str, window: Window, error: &impl std::fmt::Display) -> String {
    format!("Échec d'enregistrement du raccourci '{hotkey}' pour la fenêtre 0x{window:08x} : {error}")
}

fn msg_invalid_shortcut_window(hotkey: &str, window: Window, error: &impl std::fmt::Display) -> String {
    format!("Raccourci invalide '{hotkey}' pour la fenêtre 0x{window:08x} : {error}")
}

fn msg_register_global(action: &str, error: &impl std::fmt::Display) -> String {
    format!("Échec d'enregistrement du raccourci global pour {action} : {error}")
}

fn msg_monitor_scan_failed(error: &impl std::fmt::Display) -> String {
    format!("Échec du scan du moniteur de fenêtres : {error}")
}

fn msg_refresh_action_failed(action: &str, error: &impl std::fmt::Display) -> String {
    format!("Échec du rafraîchissement des fenêtres pour l'action {action} : {error}")
}

struct HotkeyRegistry {
    window_shortcuts: HashMap<Window, Hotkey>,
    window_hotkey_ids: HashMap<Window, HotkeyId>,
    hotkey_windows: HashMap<HotkeyId, Window>,
    hotkey_actions: HashMap<HotkeyId, HotkeyAction>,
}

impl HotkeyRegistry {
    fn new() -> Self {
        Self {
            window_shortcuts: HashMap::new(),
            window_hotkey_ids: HashMap::new(),
            hotkey_windows: HashMap::new(),
            hotkey_actions: HashMap::new(),
        }
    }

    fn find_duplicate_window(&self, hotkey: &Hotkey) -> Option<Window> {
        self.window_shortcuts
            .iter()
            .find_map(|(w, h)| (h == hotkey).then_some(*w))
    }

    fn register_window(&mut self, manager: &HotkeyManager, window: Window, hotkey: Hotkey) -> Result<HotkeyId, String> {
    match manager.register(hotkey) {
        Ok(id) => {
            self.window_shortcuts.insert(window, hotkey);
            self.window_hotkey_ids.insert(window, id);
            self.hotkey_windows.insert(id, window);
            self.hotkey_actions.insert(id, HotkeyAction::Window(window));
            Ok(id)
        }
        Err(e) => Err(format!("{e:#}")),
    }
}

    fn unregister_window(&mut self, manager: &HotkeyManager, window: Window) {
    self.window_shortcuts.remove(&window);
    if let Some(id) = self.window_hotkey_ids.remove(&window) {
        let _ = manager.unregister(id);
        self.hotkey_windows.remove(&id);
        self.hotkey_actions.remove(&id);
    }
}

    fn unregister_global(&mut self, manager: &HotkeyManager, action: HotkeyAction) {
        let id = self
            .hotkey_actions
            .iter()
            .find_map(|(id, act)| (*act == action).then_some(*id));
        if let Some(id) = id {
            let _ = manager.unregister(id);
            self.hotkey_actions.remove(&id);
            self.hotkey_windows.remove(&id);
        }
    }

    fn register_global(&mut self, manager: &HotkeyManager, action: HotkeyAction, hotkey: Hotkey) -> Result<HotkeyId, String> {
        match manager.register(hotkey) {
            Ok(id) => {
                self.hotkey_actions.insert(id, action);
                Ok(id)
            }
            Err(e) => Err(format!("{e:#}")),
        }
    }

    fn get_window_for_id(&self, id: &HotkeyId) -> Option<Window> {
        self.hotkey_windows.get(id).copied()
    }

    fn get_action_for_id(&self, id: &HotkeyId) -> Option<HotkeyAction> {
        self.hotkey_actions.get(id).copied()
    }
}

fn try_register(manager: &HotkeyManager, s: &str, action: HotkeyAction, registry: &mut HotkeyRegistry, updates_tx: &Sender<WorkerEvent>) -> Option<Hotkey> {
    if let Ok(parsed) = s.parse::<Hotkey>() {
        match registry.register_global(manager, action, parsed) {
            Ok(_id) => Some(parsed),
            Err(error) => {
                let _ = updates_tx.send(WorkerEvent::Error {
                    worker: WorkerKind::Activation,
                    message: format!("Échec de l'enregistrement du raccourci par défaut '{s}': {error}"),
                });
                None
            }
        }
    } else {
        None
    }
}

fn register_previous_candidates(manager: &HotkeyManager, registry: &mut HotkeyRegistry, updates_tx: &Sender<WorkerEvent>) -> Option<Hotkey> {
    let previous_candidates = ["²", "`", ","];
    for &cand in &previous_candidates {
        if let Some(parsed) = try_register(manager, cand, HotkeyAction::Previous, registry, updates_tx) {
            return Some(parsed);
        }
    }
    let _ = updates_tx.send(WorkerEvent::Error {
        worker: WorkerKind::Activation,
        message: "Échec de l'enregistrement des raccourcis par défaut pour Précédent (essayé : ², `, ,)".to_string(),
    });
    None
}

fn handle_next_action(conn: &impl x11rb::connection::Connection, tracker: &mut window::DofusWindowTracker, updates_tx: &Sender<WorkerEvent>) {
    match tracker.refresh(conn) {
        Ok(Some(focused)) => {
            let windows = tracker.windows().to_vec();
            if windows.is_empty() { return; }
            if let Some(pos) = windows.iter().position(|w| *w == focused) {
                let next = windows.get((pos + 1) % windows.len()).copied();
                if let Some(next_window) = next {
                    if let Err(error) = window::activation::activate(conn, next_window) {
                        let _ = updates_tx.send(WorkerEvent::Error {
                            worker: WorkerKind::Activation,
                            message: msg_activate_window(next_window, &error),
                        });
                    }
                }
            } else if let Some(first) = windows.first().copied() {
                if let Err(error) = window::activation::activate(conn, first) {
                    let _ = updates_tx.send(WorkerEvent::Error {
                        worker: WorkerKind::Activation,
                        message: msg_activate_window(first, &error),
                    });
                }
            }
        }
        Ok(None) => {
            let windows = tracker.windows().to_vec();
            if let Some(first) = windows.first().copied() {
                if let Err(error) = window::activation::activate(conn, first) {
                    let _ = updates_tx.send(WorkerEvent::Error {
                        worker: WorkerKind::Activation,
                        message: format!("Échec d'activation (première) 0x{first:08x}: {error:#}"),
                    });
                }
            }
        }
        Err(error) => {
            let _ = updates_tx.send(WorkerEvent::Error {
                worker: WorkerKind::Monitor,
                message: format!("Échec du rafraîchissement des fenêtres pour l'action Suivant: {error:#}"),
            });
        }
    }
}

fn handle_previous_action(conn: &impl x11rb::connection::Connection, tracker: &mut window::DofusWindowTracker, updates_tx: &Sender<WorkerEvent>) {
    match tracker.refresh(conn) {
        Ok(Some(focused)) => {
            let windows = tracker.windows().to_vec();
            if windows.is_empty() { return; }
            if let Some(pos) = windows.iter().position(|w| *w == focused) {
                let prev_index = if pos == 0 { windows.len().saturating_sub(1) } else { pos - 1 };
                if let Some(prev_window) = windows.get(prev_index).copied() {
                    if let Err(error) = window::activation::activate(conn, prev_window) {
                        let _ = updates_tx.send(WorkerEvent::Error {
                            worker: WorkerKind::Activation,
                            message: msg_activate_window(prev_window, &error),
                        });
                    }
                }
            } else if let Some(last) = windows.last().copied() {
                if let Err(error) = window::activation::activate(conn, last) {
                    let _ = updates_tx.send(WorkerEvent::Error {
                        worker: WorkerKind::Activation,
                        message: msg_activate_window(last, &error),
                    });
                }
            }
        }
        Ok(None) => {
            let windows = tracker.windows().to_vec();
            if let Some(last) = windows.last().copied() {
                if let Err(error) = window::activation::activate(conn, last) {
                    let _ = updates_tx.send(WorkerEvent::Error {
                        worker: WorkerKind::Activation,
                        message: format!("Échec d'activation (dernière) 0x{last:08x}: {error:#}"),
                    });
                }
            }
        }
        Err(error) => {
            let _ = updates_tx.send(WorkerEvent::Error {
                worker: WorkerKind::Monitor,
                message: format!("Échec du rafraîchissement des fenêtres pour l'action Précédent: {error:#}"),
            });
        }
    }
}

fn activate_with_report(conn: &impl x11rb::connection::Connection, window: Window, updates_tx: &Sender<WorkerEvent>) {
    if let Err(error) = window::activation::activate(conn, window) {
        let _ = updates_tx.send(WorkerEvent::Error {
            worker: WorkerKind::Activation,
            message: format!("Échec d'activation de la fenêtre 0x{window:08x} : {error:#}"),
        });
    }
}

fn handle_assign_shortcut(
    manager: &HotkeyManager,
    registry: &mut HotkeyRegistry,
    window: Window,
    hotkey_opt: Option<String>,
    updates_tx: &Sender<WorkerEvent>,
) {
    // remove if empty -> unregister
    let Some(hotkey) = hotkey_opt.filter(|v| !v.trim().is_empty()) else {
        registry.unregister_window(manager, window);
        return;
    };

    let parsed = match hotkey.trim().parse::<Hotkey>() {
        Ok(p) => p,
        Err(error) => {
            let _ = updates_tx.send(WorkerEvent::Error {
                worker: WorkerKind::Activation,
                message: format!("Raccourci invalide '{hotkey}' pour la fenêtre 0x{window:08x} : {error:#}"),
            });
            return;
        }
    };

    // unregister existing for this window
    registry.unregister_window(manager, window);

    // if another window uses same hotkey, remove it
    if let Some(duplicate_window) = registry.find_duplicate_window(&parsed) {
        registry.unregister_window(manager, duplicate_window);
    }

    match registry.register_window(manager, window, parsed) {
        Ok(_id) => {}
        Err(error) => {
            let _ = updates_tx.send(WorkerEvent::Error {
                worker: WorkerKind::Activation,
                message: format!("Échec d'enregistrement du raccourci '{hotkey}' pour la fenêtre 0x{window:08x} : {error}"),
            });
        }
    }
}

fn handle_assign_global(
    manager: &HotkeyManager,
    registry: &mut HotkeyRegistry,
    action: GlobalAction,
    hotkey_opt: Option<String>,
    updates_tx: &Sender<WorkerEvent>,
) -> Option<Hotkey> {
    let target_action = match action {
        GlobalAction::Next => HotkeyAction::Next,
        GlobalAction::Previous => HotkeyAction::Previous,
    };

    if hotkey_opt.as_ref().map(|s| s.trim().is_empty()).unwrap_or(true) {
        registry.unregister_global(manager, target_action);
        return None;
    }

    let parsed = match hotkey_opt.unwrap().trim().parse::<Hotkey>() {
        Ok(p) => p,
        Err(error) => {
            let _ = updates_tx.send(WorkerEvent::Error {
                worker: WorkerKind::Activation,
                message: format!("Raccourci global invalide pour {action:?} : {error:#}"),
            });
            return None;
        }
    };

    // avoid collision with window-specific shortcuts
    if let Some(duplicate_window) = registry.find_duplicate_window(&parsed) {
        registry.unregister_window(manager, duplicate_window);
    }

    registry.unregister_global(manager, target_action);
    match registry.register_global(manager, target_action, parsed) {
        Ok(_id) => Some(parsed),
        Err(error) => {
            let _ = updates_tx.send(WorkerEvent::Error {
                worker: WorkerKind::Activation,
                message: format!("Échec d'enregistrement du raccourci global pour {action:?} : {error}"),
            });
            None
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
                message: format!("Échec de la connexion du moniteur de fenêtres à X11 : {error}"),
            });
            return;
        }
    };

    let manager = match HotkeyManager::new() {
        Ok(manager) => manager,
        Err(error) => {
            let _ = updates_tx.send(WorkerEvent::Error {
                worker: WorkerKind::Activation,
                message: format!("Échec de l'initialisation du gestionnaire de raccourcis globaux : {error:#}"),
            });
            return;
        }
    };

    let mut registry = HotkeyRegistry::new();
    let mut _next_hotkey: Option<Hotkey> = None;
    let mut _previous_hotkey: Option<Hotkey> = None;
    let mut tracker = window::DofusWindowTracker::new();
    let mut last_scan_error = None;

    // Register default global shortcuts: Tab -> Next, ² -> Previous
    _next_hotkey = try_register(&manager, "Tab", HotkeyAction::Next, &mut registry, &updates_tx);
    _previous_hotkey = register_previous_candidates(&manager, &mut registry, &updates_tx);

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
                            message: msg_activate_window(window, &error),
                        });
                    }
                }
                WindowCommand::AssignShortcut { window, hotkey } => {
                    handle_assign_shortcut(&manager, &mut registry, window, hotkey, &updates_tx);
                }
                WindowCommand::AssignGlobal { action, hotkey } => {
                    match action {
                        GlobalAction::Next => {
                            _next_hotkey = handle_assign_global(&manager, &mut registry, GlobalAction::Next, hotkey, &updates_tx);
                        }
                        GlobalAction::Previous => {
                            _previous_hotkey = handle_assign_global(&manager, &mut registry, GlobalAction::Previous, hotkey, &updates_tx);
                        }
                    }
                }
            }
        }

        while let Some(event) = manager.try_recv() {
            if event.state != HotkeyState::Pressed {
                continue;
            }

            if let Some(target_window) = registry.get_window_for_id(&event.id) {
                if let Err(error) = window::activation::activate(&conn, target_window) {
                    let _ = updates_tx.send(WorkerEvent::Error {
                        worker: WorkerKind::Activation,
                        message: msg_activate_window(target_window, &error),
                    });
                }
                continue;
            }

            if let Some(action) = registry.get_action_for_id(&event.id) {
                match action {
                    HotkeyAction::Next => {
                        match tracker.refresh(&conn) {
                            Ok(Some(focused)) => {
                                let windows = tracker.windows().to_vec();
                                if windows.is_empty() { continue; }
                                if let Some(pos) = windows.iter().position(|w| *w == focused) {
                                    let next = windows.get((pos + 1) % windows.len()).copied();
                                    if let Some(next_window) = next && let Err(error) = window::activation::activate(&conn, next_window) {
                                        let _ = updates_tx.send(WorkerEvent::Error {
                                            worker: WorkerKind::Activation,
                                            message: msg_activate_window(next_window, &error),
                                        });
                                    }
                                } else if let Some(first) = windows.first().copied() && let Err(error) = window::activation::activate(&conn, first) {
                                let _ = updates_tx.send(WorkerEvent::Error {
                                    worker: WorkerKind::Activation,
                                    message: msg_activate_window(first, &error),
                                });
                                }
                            }
                            Ok(None) => {
                                // No window currently focused: activate the first detected window (wrap to start)
                                let windows = tracker.windows().to_vec();
                                if let Some(first) = windows.first().copied() && let Err(error) = window::activation::activate(&conn, first) {
                                    let _ = updates_tx.send(WorkerEvent::Error {
                                        worker: WorkerKind::Activation,
                                        message: msg_activate_window(first, &error),
                                    });
                                }
                            }
                            Err(error) => {
                                let _ = updates_tx.send(WorkerEvent::Error {
                                    worker: WorkerKind::Monitor,
                                    message: msg_refresh_action_failed("Suivant", &error),
                                });
                            }
                        }
                    }
                    HotkeyAction::Previous => {
                            match tracker.refresh(&conn) {
                                Ok(Some(focused)) => {
                                    let windows = tracker.windows().to_vec();
                                    if windows.is_empty() { continue; }
                                    if let Some(pos) = windows.iter().position(|w| *w == focused) {
                                        let prev_index = if pos == 0 { windows.len().saturating_sub(1) } else { pos - 1 };
                                        if let Some(prev_window) = windows.get(prev_index).copied() && let Err(error) = window::activation::activate(&conn, prev_window) {
                                            let _ = updates_tx.send(WorkerEvent::Error {
                                                worker: WorkerKind::Activation,
                                                message: msg_activate_window(prev_window, &error),
                                            });
                                        }
                                    } else if let Some(last) = windows.last().copied() && let Err(error) = window::activation::activate(&conn, last) {
                                    let _ = updates_tx.send(WorkerEvent::Error {
                                        worker: WorkerKind::Activation,
                                        message: msg_activate_window(last, &error),
                                    });
                                }
                                }
                                Ok(None) => {
                                    // No window currently focused: activate the last detected window (wrap to end)
                                    let windows = tracker.windows().to_vec();
                                    if let Some(last) = windows.last().copied() && let Err(error) = window::activation::activate(&conn, last) {
                                        let _ = updates_tx.send(WorkerEvent::Error {
                                            worker: WorkerKind::Activation,
                                            message: msg_activate_window(last, &error),
                                        });
                                    }
                                }
                                Err(error) => {
                                    let _ = updates_tx.send(WorkerEvent::Error {
                                        worker: WorkerKind::Monitor,
                                        message: msg_refresh_action_failed("Précédent", &error),
                                    });
                                }
                            }
                    }
                    HotkeyAction::Window(_) => { /* handled above */ }
                }
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
                let message = msg_monitor_scan_failed(&error);
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
