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
                if let Some(manager) = &state.manager {
                    handle_assign_shortcut(manager, &mut state.registry, window, hotkey, updates);
                }
            }
            WindowCommand::AssignGlobal { action, hotkey } => {
                if let Some(manager) = &state.manager {
                    let registered =
                        handle_assign_global(manager, &mut state.registry, action, hotkey, updates);
                    match action {
                        GlobalAction::Next => state._next_hotkey = registered,
                        GlobalAction::Previous => state._previous_hotkey = registered,
                    }
                }
            }
            WindowCommand::SetRestrictPrevNextToDofus(value) => {
                state.restrict_prev_next_to_dofus = value;
            }
            WindowCommand::SetWindowOrder(order) => state.window_order = order,
        }
    }
}
