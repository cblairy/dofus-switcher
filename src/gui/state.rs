use std::collections::HashMap;

use x11rb::protocol::xproto::Window;

#[derive(Default)]
pub(super) struct WindowListState {
    pub(super) windows: Vec<Window>,
    pub(super) focused_window: Option<Window>,
    pub(super) selected_window: Option<Window>,
    pub(super) character_names: HashMap<Window, String>,
    pub(super) errors: HashMap<crate::runtime::WorkerKind, String>,
}

impl WindowListState {
    pub(super) fn apply_windows(&mut self, windows: Vec<Window>, focused_window: Option<Window>) {
        self.windows = windows;
        self.focused_window = focused_window;
        self.character_names
            .retain(|window, _| self.windows.contains(window));
        if self
            .selected_window
            .is_some_and(|window| !self.windows.contains(&window))
        {
            self.selected_window = None;
        }
    }

    pub(super) fn character_label(&self, window: Window) -> &str {
        self.character_names
            .get(&window)
            .filter(|name| !name.trim().is_empty())
            .map(String::as_str)
            .unwrap_or("Unassigned")
    }
}

#[cfg(test)]
mod tests {
    use super::WindowListState;
    use std::collections::HashMap;

    #[test]
    fn window_update_prunes_removed_window_state() {
        let mut state = WindowListState {
            windows: vec![1, 2],
            focused_window: Some(2),
            selected_window: Some(2),
            character_names: HashMap::from([(1, "Alya".to_owned()), (2, "Boris".to_owned())]),
            errors: HashMap::new(),
        };

        state.apply_windows(vec![1, 3], Some(3));

        assert_eq!(state.windows, vec![1, 3]);
        assert_eq!(state.focused_window, Some(3));
        assert_eq!(state.selected_window, None);
        assert_eq!(
            state.character_names,
            HashMap::from([(1, "Alya".to_owned())])
        );
    }

    #[test]
    fn window_update_preserves_names_and_selection_for_detected_windows() {
        let mut state = WindowListState {
            windows: vec![1, 2],
            focused_window: None,
            selected_window: Some(2),
            character_names: HashMap::from([(2, "Boris".to_owned())]),
            errors: HashMap::new(),
        };

        state.apply_windows(vec![2, 1], Some(1));

        assert_eq!(state.selected_window, Some(2));
        assert_eq!(
            state.character_names.get(&2).map(String::as_str),
            Some("Boris")
        );
    }

    #[test]
    fn character_label_uses_assigned_name_or_unassigned_fallback() {
        let state = WindowListState {
            character_names: HashMap::from([(1, "  Alya  ".to_owned()), (2, "  ".to_owned())]),
            ..Default::default()
        };

        assert_eq!(state.character_label(1), "  Alya  ");
        assert_eq!(state.character_label(2), "Unassigned");
        assert_eq!(state.character_label(3), "Unassigned");
    }
}
