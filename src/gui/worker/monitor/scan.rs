use std::sync::mpsc::Sender;

use x11rb::protocol::xproto::Window;

use crate::runtime::{WorkerEvent, WorkerKind};

use super::state::MonitorState;

pub(super) fn publish(
    state: &mut MonitorState,
    updates: &Sender<WorkerEvent>,
    last_error: &mut Option<String>,
) -> bool {
    match state.tracker.refresh(&state.connection) {
        Ok(focused_window) => {
            if last_error.take().is_some()
                && updates
                    .send(WorkerEvent::Recovered(WorkerKind::Monitor))
                    .is_err()
            {
                return false;
            }
            updates
                .send(WorkerEvent::Windows {
                    windows: state.tracker.windows().to_vec(),
                    focused_window,
                })
                .is_ok()
        }
        Err(error) => {
            let message = format!("Échec du scan du moniteur de fenêtres : {error}");
            if last_error.as_deref() == Some(message.as_str()) {
                return true;
            }
            if updates
                .send(WorkerEvent::Error {
                    worker: WorkerKind::Monitor,
                    message: message.clone(),
                })
                .is_err()
            {
                return false;
            }
            *last_error = Some(message);
            true
        }
    }
}

pub(super) fn ordered_windows(detected: &[Window], preferred: &[Window]) -> Vec<Window> {
    let mut ordered: Vec<_> = preferred
        .iter()
        .copied()
        .filter(|window| detected.contains(window))
        .collect();
    for window in detected {
        if !ordered.contains(window) {
            ordered.push(*window);
        }
    }
    ordered
}
