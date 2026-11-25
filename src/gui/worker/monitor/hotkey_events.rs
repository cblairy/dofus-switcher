use handy_keys::HotkeyState;

use crate::runtime::{WorkerEvent, WorkerKind};
use crate::window;

use super::state::MonitorState;

pub(super) fn process(state: &mut MonitorState, updates: &std::sync::mpsc::Sender<WorkerEvent>) {
    while let Some(event) = state.manager.try_recv() {
        if event.state != HotkeyState::Pressed {
            continue;
        }
        if let Some(window) = state.registry.get_window_for_id(&event.id) {
            super::activate(window, state, updates);
            continue;
        }
        match state.registry.get_action_for_id(&event.id) {
            Some(crate::gui::worker::hotkeys::HotkeyAction::Next) => {
                if state.restrict_prev_next_to_dofus {
                    match window::active_window(&state.connection, state.tracker.windows()) {
                        Ok(None) => continue,
                        Ok(Some(_)) => {}
                        Err(error) => {
                            send_error(
                                updates,
                                WorkerKind::Monitor,
                                format!("Échec de vérification du focus Dofus : {error:#}"),
                            );
                            continue;
                        }
                    }
                }
                activate_next(state, updates);
            }
            Some(crate::gui::worker::hotkeys::HotkeyAction::Previous) => {
                activate_previous(state, updates);
            }
            _ => {}
        }
    }
}

fn activate_next(state: &mut MonitorState, updates: &std::sync::mpsc::Sender<WorkerEvent>) {
    match state.tracker.refresh(&state.connection) {
        Ok(focused) => {
            let windows =
                super::scan::ordered_windows(state.tracker.windows(), &state.window_order);
            if windows.is_empty() {
                return;
            }
            let next = focused
                .and_then(|current| windows.iter().position(|window| *window == current))
                .map(|position| windows[(position + 1) % windows.len()])
                .or_else(|| windows.first().copied());
            if let Some(window) = next {
                super::activate(window, state, updates);
            }
        }
        Err(error) => send_error(
            updates,
            WorkerKind::Monitor,
            format!("Échec du rafraîchissement des fenêtres pour l'action Suivant : {error:#}"),
        ),
    }
}

fn activate_previous(state: &mut MonitorState, updates: &std::sync::mpsc::Sender<WorkerEvent>) {
    match state.tracker.refresh(&state.connection) {
        Ok(focused) => {
            let windows =
                super::scan::ordered_windows(state.tracker.windows(), &state.window_order);
            if windows.is_empty() {
                return;
            }
            let previous = focused
                .and_then(|current| windows.iter().position(|window| *window == current))
                .map(|position| {
                    windows[if position == 0 {
                        windows.len() - 1
                    } else {
                        position - 1
                    }]
                })
                .or_else(|| windows.last().copied());
            if let Some(window) = previous {
                super::activate(window, state, updates);
            }
        }
        Err(error) => send_error(
            updates,
            WorkerKind::Monitor,
            format!("Échec du rafraîchissement des fenêtres pour l'action Précédent : {error:#}"),
        ),
    }
}

fn send_error(updates: &std::sync::mpsc::Sender<WorkerEvent>, worker: WorkerKind, message: String) {
    let _ = updates.send(WorkerEvent::Error { worker, message });
}
