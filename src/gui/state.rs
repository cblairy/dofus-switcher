use std::collections::HashMap;

use x11rb::protocol::xproto::Window;

/// Actions for global shortcuts handled by the worker
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum GlobalAction {
    Next,
    Previous,
}

pub(super) struct WindowListState {
    pub(super) windows: Vec<Window>,
    pub(super) detected_windows: Vec<Window>,
    pub(super) focused_window: Option<Window>,
    pub(super) selected_window: Option<Window>,
    pub(super) character_names: HashMap<Window, String>,
    pub(super) shortcut_keys: HashMap<Window, String>,
    pub(super) next_key: Option<String>,
    pub(super) previous_key: Option<String>,
    pub(super) capture_window: Option<Window>,
    pub(super) capture_global: Option<GlobalAction>,
    pub(super) errors: HashMap<crate::runtime::WorkerKind, String>,
    pub(super) restrict_prev_next_to_dofus: bool,
    pub(super) persisted_characters: Vec<crate::config::CharacterConfig>,
    pub(super) persisted_order: Vec<usize>,
    pub(super) initial_order_applied: bool,
}

impl Default for WindowListState {
    fn default() -> Self {
        Self {
            windows: Vec::new(),
            detected_windows: Vec::new(),
            focused_window: None,
            selected_window: None,
            character_names: HashMap::new(),
            shortcut_keys: HashMap::new(),
            // Defaults requested: Tab for Next, try '²' for Previous but fall back to ` if needed
            next_key: Some("Tab".to_owned()),
            previous_key: Some("`".to_owned()),
            capture_window: None,
            capture_global: None,
            errors: HashMap::new(),
            restrict_prev_next_to_dofus: true,
            persisted_characters: Vec::new(),
            persisted_order: Vec::new(),
            initial_order_applied: false,
        }
    }
}

impl WindowListState {
    pub(super) fn apply_windows(&mut self, windows: Vec<Window>, focused_window: Option<Window>) {
        self.detected_windows = windows;
        self.reconcile_order();
        self.focused_window = focused_window;
        self.character_names
            .retain(|window, _| self.detected_windows.contains(window));
        self.shortcut_keys
            .retain(|window, _| self.detected_windows.contains(window));
        if self
            .selected_window
            .is_some_and(|window| !self.detected_windows.contains(&window))
        {
            self.selected_window = None;
        }
        if self
            .capture_window
            .is_some_and(|window| !self.detected_windows.contains(&window))
        {
            self.capture_window = None;
        }
        self.apply_persisted_characters();
    }

    pub(super) fn apply_persisted_characters(&mut self) {
        if self.persisted_characters.is_empty() {
            return;
        }

        for (index, window) in self.detected_windows.iter().enumerate() {
            if let Some(entry) = self.persisted_characters.get(index) {
                if !entry.name.trim().is_empty() {
                    self.character_names.insert(*window, entry.name.clone());
                }
                if let Some(shortcut) = &entry.shortcut {
                    self.shortcut_keys.insert(*window, shortcut.clone());
                }
            }
        }
        self.persisted_characters.clear();
    }

    fn reconcile_order(&mut self) {
        self.windows
            .retain(|window| self.detected_windows.contains(window));
        for window in &self.detected_windows {
            if !self.windows.contains(window) {
                self.windows.push(*window);
            }
        }
        if !self.initial_order_applied {
            let current = self.windows.clone();
            let mut ordered = Vec::new();
            for index in &self.persisted_order {
                if let Some(window) = current.get(*index) {
                    if !ordered.contains(window) {
                        ordered.push(*window);
                    }
                }
            }
            for window in current {
                if !ordered.contains(&window) {
                    ordered.push(window);
                }
            }
            self.windows = ordered;
            self.initial_order_applied = true;
        }
    }

    pub(super) fn move_window(&mut self, window: Window, offset: isize) {
        let Some(index) = self
            .windows
            .iter()
            .position(|candidate| *candidate == window)
        else {
            return;
        };
        let target = index as isize + offset;
        if !(0..self.windows.len() as isize).contains(&target) {
            return;
        }
        self.windows.swap(index, target as usize);
    }

    pub(super) fn assign_shortcut(&mut self, window: Window, key: Option<String>) {
        self.shortcut_keys.retain(|mapped_window, mapped_key| {
            *mapped_window != window && Some(mapped_key.clone()) != key
        });
        if let Some(key) = key {
            self.shortcut_keys.insert(window, key);
        }
    }

    pub(super) fn character_label(&self, window: Window) -> &str {
        self.character_names
            .get(&window)
            .filter(|name| !name.trim().is_empty())
            .map(String::as_str)
            .unwrap_or("Non assigné")
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
            shortcut_keys: HashMap::from([(1, "1".to_owned()), (2, "2".to_owned())]),
            next_key: None,
            previous_key: None,
            capture_window: None,
            capture_global: None,
            errors: HashMap::new(),
            restrict_prev_next_to_dofus: true,
            persisted_characters: Vec::new(),
            persisted_order: Vec::new(),
            initial_order_applied: false,
            detected_windows: vec![1, 2],
        };
        state.apply_windows(vec![1, 3], Some(3));

        assert_eq!(state.windows, vec![1, 3]);
        assert_eq!(state.focused_window, Some(3));
        assert_eq!(state.selected_window, None);
        assert_eq!(
            state.character_names,
            HashMap::from([(1, "Alya".to_owned())])
        );
        assert_eq!(state.shortcut_keys, HashMap::from([(1, "1".to_owned())]));
    }

    #[test]
    fn window_update_preserves_names_and_selection_for_detected_windows() {
        let mut state = WindowListState {
            windows: vec![1, 2],
            focused_window: None,
            selected_window: Some(2),
            character_names: HashMap::from([(2, "Boris".to_owned())]),
            shortcut_keys: HashMap::from([(2, "3".to_owned())]),
            next_key: None,
            previous_key: None,
            capture_window: None,
            capture_global: None,
            errors: HashMap::new(),
            restrict_prev_next_to_dofus: true,
            persisted_characters: Vec::new(),
            detected_windows: vec![1, 2],
            persisted_order: Vec::new(),
            initial_order_applied: false,
        };
        state.apply_windows(vec![2, 1], Some(1));

        assert_eq!(state.selected_window, Some(2));
        assert_eq!(
            state.character_names.get(&2).map(String::as_str),
            Some("Boris")
        );
        assert_eq!(state.shortcut_keys.get(&2), Some(&"3".to_owned()));
    }

    #[test]
    fn assigning_shortcut_moves_it_from_previous_window() {
        let mut state = WindowListState {
            shortcut_keys: HashMap::from([(1, "1".to_owned()), (2, "2".to_owned())]),
            next_key: None,
            previous_key: None,
            capture_window: None,
            capture_global: None,
            errors: HashMap::new(),
            windows: Vec::new(),
            focused_window: None,
            selected_window: None,
            character_names: HashMap::new(),
            restrict_prev_next_to_dofus: true,
            persisted_characters: Vec::new(),
            detected_windows: Vec::new(),
            persisted_order: Vec::new(),
            initial_order_applied: false,
        };

        state.assign_shortcut(2, Some("1".to_owned()));

        assert_eq!(state.shortcut_keys, HashMap::from([(2, "1".to_owned())]));
    }

    #[test]
    fn shortcut_can_be_removed() {
        let mut state = WindowListState::default();

        state.assign_shortcut(1, Some("Ctrl+Alt+K".to_owned()));
        state.assign_shortcut(1, None);
        assert!(state.shortcut_keys.is_empty());
    }

    #[test]
    fn character_label_uses_assigned_name_or_unassigned_fallback() {
        let state = WindowListState {
            character_names: HashMap::from([(1, "  Alya  ".to_owned()), (2, "  ".to_owned())]),
            ..Default::default()
        };

        assert_eq!(state.character_label(1), "  Alya  ");
        assert_eq!(state.character_label(2), "Non assigné");
        assert_eq!(state.character_label(3), "Non assigné");
    }
}
