use std::collections::HashMap;

use x11rb::protocol::xproto::Window;

#[derive(Default)]
pub(super) struct WindowListState {
    pub(super) windows: Vec<Window>,
    pub(super) errors: HashMap<crate::runtime::WorkerKind, String>,
}

#[cfg(test)]
mod tests {
    use super::WindowListState;
    use crate::runtime::WorkerEvent;
    use std::collections::HashMap;

    #[test]
    fn window_update_replaces_window_list() {
        let mut state = WindowListState {
            windows: vec![1, 2],
            errors: HashMap::new(),
        };
        let update = WorkerEvent::Windows(vec![3]);

        if let WorkerEvent::Windows(windows) = update {
            state.windows = windows;
        }

        assert_eq!(state.windows, vec![3]);
        assert_eq!(state.errors, HashMap::new());
    }
}
