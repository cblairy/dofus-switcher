use std::sync::mpsc::Sender;

use handy_keys::{Hotkey, HotkeyManager};
use x11rb::protocol::xproto::Window;

use crate::gui::state::GlobalAction;
use crate::runtime::{WorkerEvent, WorkerKind};

use super::registry::{HotkeyAction, HotkeyRegistry};

pub(crate) fn try_register(
    manager: &HotkeyManager,
    s: &str,
    action: HotkeyAction,
    registry: &mut HotkeyRegistry,
    updates_tx: &Sender<WorkerEvent>,
) -> Option<Hotkey> {
    if let Ok(parsed) = s.parse::<Hotkey>() {
        match registry.register_global(manager, action, parsed) {
            Ok(_id) => Some(parsed),
            Err(error) => {
                let _ = updates_tx.send(WorkerEvent::Error {
                    worker: WorkerKind::Activation,
                    message: format!(
                        "Échec de l'enregistrement du raccourci par défaut '{s}': {error}"
                    ),
                });
                None
            }
        }
    } else {
        None
    }
}

pub(crate) fn register_previous_candidates(
    manager: &HotkeyManager,
    registry: &mut HotkeyRegistry,
    updates_tx: &Sender<WorkerEvent>,
) -> Option<Hotkey> {
    let previous_candidates = ["²", "`", ","];
    for &cand in &previous_candidates {
        if let Some(parsed) =
            try_register(manager, cand, HotkeyAction::Previous, registry, updates_tx)
        {
            return Some(parsed);
        }
    }
    let _ = updates_tx.send(WorkerEvent::Error {
        worker: WorkerKind::Activation,
        message:
            "Échec de l'enregistrement des raccourcis par défaut pour Précédent (essayé : ², `, ,)"
                .to_string(),
    });
    None
}

pub(crate) fn handle_assign_shortcut(
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
                message: format!(
                    "Raccourci invalide '{hotkey}' pour la fenêtre 0x{window:08x} : {error:#}"
                ),
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

pub(crate) fn handle_assign_global(
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

    if hotkey_opt
        .as_ref()
        .map(|s| s.trim().is_empty())
        .unwrap_or(true)
    {
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
                message: format!(
                    "Échec d'enregistrement du raccourci global pour {action:?} : {error}"
                ),
            });
            None
        }
    }
}
