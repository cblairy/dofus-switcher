mod handle;
mod hotkeys;
mod monitor;

pub(crate) use handle::WorkerHandle;

use crate::gui::state::GlobalAction;
use x11rb::protocol::xproto::Window;

pub(super) enum WindowCommand {
    Activate(Window),
    AssignShortcut {
        window: Window,
        hotkey: Option<String>,
    },
    AssignGlobal {
        action: GlobalAction,
        hotkey: Option<String>,
    },
    SetRestrictPrevNextToDofus(bool),
    SetWindowOrder(Vec<Window>),
}
