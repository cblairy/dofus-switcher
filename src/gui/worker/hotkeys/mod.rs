mod registration;
mod registry;

pub(crate) use registration::{
    handle_assign_global, handle_assign_shortcut, register_previous_candidates, try_register,
};
pub(crate) use registry::{HotkeyAction, HotkeyRegistry};
