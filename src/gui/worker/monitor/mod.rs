mod commands;
mod hotkey_events;
mod scan;
mod state;

use std::sync::mpsc::{Receiver, Sender};

use super::WindowCommand;
use crate::runtime::WorkerEvent;
use state::MonitorState;

pub(super) fn window_monitor(
    updates_tx: Sender<WorkerEvent>,
    commands_rx: Receiver<WindowCommand>,
    stop_rx: Receiver<()>,
) {
    let Some(mut state) = MonitorState::connect(&updates_tx) else {
        return;
    };
    state.register_default_hotkeys(&updates_tx);
    let mut last_scan_error = None;

    loop {
        if stop_rx.try_recv().is_ok() {
            break;
        }
        commands::process(&mut state, &commands_rx, &updates_tx);
        hotkey_events::process(&mut state, &updates_tx);
        if !scan::publish(&mut state, &updates_tx, &mut last_scan_error) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
}

fn activate(window: u32, state: &MonitorState, updates: &Sender<WorkerEvent>) {
    if let Err(error) = crate::window::activation::activate(&state.connection, window) {
        let _ = updates.send(WorkerEvent::Error {
            worker: crate::runtime::WorkerKind::Activation,
            message: format!("Échec d'activation de la fenêtre 0x{window:08x} : {error:#}"),
        });
    }
}
