use handy_keys::{Hotkey, HotkeyManager};
use x11rb::protocol::xproto::Window;
use x11rb::rust_connection::RustConnection;

use super::super::hotkeys::{
    HotkeyAction, HotkeyRegistry, register_previous_candidates, try_register,
};
use crate::runtime::{WorkerEvent, WorkerKind};
use crate::window::DofusWindowTracker;

pub(super) struct MonitorState {
    pub(super) connection: RustConnection,
    pub(super) manager: Option<HotkeyManager>,
    pub(super) registry: HotkeyRegistry,
    pub(super) tracker: DofusWindowTracker,
    pub(super) window_order: Vec<Window>,
    pub(super) restrict_prev_next_to_dofus: bool,
    pub(super) _next_hotkey: Option<Hotkey>,
    pub(super) _previous_hotkey: Option<Hotkey>,
}

impl MonitorState {
    pub(super) fn connect(updates: &std::sync::mpsc::Sender<WorkerEvent>) -> Option<Self> {
        let (connection, _) = match x11rb::connect(None) {
            Ok(connection) => connection,
            Err(error) => {
                let _ = updates.send(WorkerEvent::Error {
                    worker: WorkerKind::Monitor,
                    message: format!(
                        "Échec de la connexion du moniteur de fenêtres à X11 : {error}"
                    ),
                });
                return None;
            }
        };
        let manager = match HotkeyManager::new() {
            Ok(manager) => Some(manager),
            Err(error) => {
                let _ = updates.send(WorkerEvent::Error {
                    worker: WorkerKind::Activation,
                    message: format!("Raccourcis globaux indisponibles : impossible d’accéder à /dev/input ({error:#}). La détection des fenêtres reste active ; vérifiez les permissions du périphérique clavier."),
                });
                None
            }
        };
        Some(Self {
            connection,
            manager,
            registry: HotkeyRegistry::new(),
            tracker: DofusWindowTracker::new(),
            window_order: Vec::new(),
            restrict_prev_next_to_dofus: true,
            _next_hotkey: None,
            _previous_hotkey: None,
        })
    }

    pub(super) fn register_default_hotkeys(
        &mut self,
        updates: &std::sync::mpsc::Sender<WorkerEvent>,
    ) {
        let Some(manager) = &self.manager else {
            return;
        };
        self._next_hotkey = try_register(
            manager,
            "Tab",
            HotkeyAction::Next,
            &mut self.registry,
            updates,
        );
        self._previous_hotkey = register_previous_candidates(manager, &mut self.registry, updates);
    }
}
