mod commands;
mod hotkey_events;
mod scan;
mod state;

use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use super::WindowCommand;
use crate::runtime::WorkerEvent;
use state::MonitorState;

const COMMAND_POLL_INTERVAL: Duration = Duration::from_millis(10);
const WINDOW_SCAN_INTERVAL: Duration = Duration::from_millis(100);

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
    let mut next_window_scan = Instant::now();

    loop {
        if matches!(
            stop_rx.try_recv(),
            Ok(()) | Err(std::sync::mpsc::TryRecvError::Disconnected)
        ) {
            break;
        }
        commands::process(&mut state, &commands_rx, &updates_tx);
        hotkey_events::process(&mut state, &updates_tx);
        if Instant::now() >= next_window_scan {
            if !scan::publish(&mut state, &updates_tx, &mut last_scan_error) {
                break;
            }
            next_window_scan = Instant::now() + WINDOW_SCAN_INTERVAL;
        }
        match stop_rx.recv_timeout(COMMAND_POLL_INTERVAL) {
            Ok(()) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
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
