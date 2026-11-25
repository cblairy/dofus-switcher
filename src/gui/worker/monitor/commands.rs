use std::sync::mpsc::{Receiver, Sender};

use crate::gui::state::GlobalAction;
use crate::runtime::WorkerEvent;

use super::super::WindowCommand;
use super::super::hotkeys::{handle_assign_global, handle_assign_shortcut};
use super::state::MonitorState;

pub(super) fn process(
    state: &mut MonitorState,
    commands: &Receiver<WindowCommand>,
    updates: &Sender<WorkerEvent>,
) {
    while let Ok(command) = commands.try_recv() {
        match command {
            WindowCommand::Activate(window) => super::activate(window, state, updates),
            WindowCommand::AssignShortcut { window, hotkey } => {
                handle_assign_shortcut(
                    &state.manager,
                    &mut state.registry,
                    window,
                    hotkey,
                    updates,
                );
            }
            WindowCommand::AssignGlobal { action, hotkey } => {
                let registered = handle_assign_global(
                    &state.manager,
                    &mut state.registry,
                    action,
                    hotkey,
                    updates,
                );
                match action {
                    GlobalAction::Next => state._next_hotkey = registered,
                    GlobalAction::Previous => state._previous_hotkey = registered,
                }
            }
            WindowCommand::SetRestrictPrevNextToDofus(value) => {
                state.restrict_prev_next_to_dofus = value;
            }
            WindowCommand::SetWindowOrder(order) => state.window_order = order,
        }
    }
}
