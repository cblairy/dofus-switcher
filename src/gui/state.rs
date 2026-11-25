use std::collections::{HashMap, HashSet};

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
    pub(super) saved_window_assignments: HashMap<u32, String>,
    pub(super) initiative_order: Vec<String>,
    pub(super) auto_order_restore_pending: bool,
    pub(super) association_window: Option<Window>,
    pub(super) association_query: String,
    pub(super) ocr_region: Option<crate::config::OcrRegion>,
    pub(super) ocr_text: String,
    pub(super) ocr_target: Option<Window>,
    pub(super) ocr_candidate: Option<Window>,
    pub(super) ocr_candidate_hits: u8,
    pub(super) ocr_candidate_switched: bool,
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
            saved_window_assignments: HashMap::new(),
            initiative_order: Vec::new(),
            auto_order_restore_pending: false,
            association_window: None,
            association_query: String::new(),
            ocr_region: None,
            ocr_text: String::new(),
            ocr_target: None,
            ocr_candidate: None,
            ocr_candidate_hits: 0,
            ocr_candidate_switched: false,
        }
    }
}

impl WindowListState {
    pub(super) fn apply_windows(&mut self, windows: Vec<Window>, focused_window: Option<Window>) {
        self.detected_windows = windows;
        let restored_names: Vec<_> = self
            .detected_windows
            .iter()
            .filter_map(|window| self.saved_window_assignments.get(window))
            .collect();
        let auto_assignment_complete = !self.detected_windows.is_empty()
            && self.detected_windows.len() == self.saved_window_assignments.len()
            && self.detected_windows.iter().all(|window| {
                self.saved_window_assignments
                    .get(window)
                    .is_some_and(|name| {
                        self.initiative_order.contains(name)
                            && self
                                .persisted_characters
                                .iter()
                                .any(|character| character.name == *name)
                    })
            })
            && restored_names
                .iter()
                .map(|name| name.to_lowercase())
                .collect::<HashSet<_>>()
                .len()
                == restored_names.len();
        self.reconcile_order();
        self.focused_window = focused_window;
        self.character_names
            .retain(|window, _| self.detected_windows.contains(window));
        self.saved_window_assignments
            .retain(|window, _| self.detected_windows.contains(window));
        self.shortcut_keys
            .retain(|window, _| self.detected_windows.contains(window));
        let restored_assignments: Vec<_> = self
            .detected_windows
            .iter()
            .filter(|window| !self.character_names.contains_key(window))
            .filter_map(|window| {
                self.saved_window_assignments
                    .get(window)
                    .map(|name| (*window, name.clone()))
            })
            .collect();
        for (window, name) in restored_assignments {
            self.assign_character(window, name);
        }
        if self.auto_order_restore_pending {
            self.auto_order_restore_pending = false;
            if auto_assignment_complete {
                self.sort_windows_by_initiative_order();
            }
        }
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
        if self
            .association_window
            .is_some_and(|window| !self.detected_windows.contains(&window))
        {
            self.association_window = None;
            self.association_query.clear();
        }
    }

    fn reconcile_order(&mut self) {
        self.windows
            .retain(|window| self.detected_windows.contains(window));
        for window in &self.detected_windows {
            if !self.windows.contains(window) {
                self.windows.push(*window);
            }
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
        self.update_initiative_order_from_windows();
    }

    pub(super) fn assign_shortcut(&mut self, window: Window, key: Option<String>) {
        self.shortcut_keys.retain(|mapped_window, mapped_key| {
            *mapped_window != window && Some(mapped_key.clone()) != key
        });
        if let Some(key) = key {
            self.shortcut_keys.insert(window, key);
        }
    }

    pub(super) fn assign_character(&mut self, window: Window, name: String) {
        let Some(character) = self
            .persisted_characters
            .iter()
            .find(|character| character.name.eq_ignore_ascii_case(&name))
            .cloned()
        else {
            return;
        };

        self.assign_character_profile(window, character);
    }

    pub(super) fn create_character(&mut self, window: Window, name: String) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if self
            .persisted_characters
            .iter()
            .any(|character| character.name.eq_ignore_ascii_case(name))
        {
            self.assign_character(window, name.to_owned());
            return;
        }
        let character = crate::config::CharacterConfig {
            name: name.to_owned(),
            shortcut: None,
        };
        self.persisted_characters.push(character.clone());
        self.assign_character_profile(window, character);
    }

    pub(super) fn delete_character(&mut self, name: &str) {
        self.persisted_characters
            .retain(|character| !character.name.eq_ignore_ascii_case(name));
        let assigned_windows: Vec<_> = self
            .character_names
            .iter()
            .filter_map(|(window, assigned)| assigned.eq_ignore_ascii_case(name).then_some(*window))
            .collect();
        for window in assigned_windows {
            self.character_names.remove(&window);
            self.shortcut_keys.remove(&window);
        }
        self.saved_window_assignments
            .retain(|_, assigned| !assigned.eq_ignore_ascii_case(name));
        self.initiative_order
            .retain(|entry| !entry.eq_ignore_ascii_case(name));
    }

    fn assign_character_profile(
        &mut self,
        window: Window,
        character: crate::config::CharacterConfig,
    ) {
        for (other_window, other_name) in &mut self.character_names {
            if *other_window != window && *other_name == character.name {
                other_name.clear();
                self.shortcut_keys.remove(other_window);
            }
        }
        self.character_names.insert(window, character.name);
        self.assign_shortcut(window, character.shortcut);
    }

    pub(super) fn sync_character_profiles(&mut self) {
        let mut profiles = Vec::new();
        let mut window_assignments = HashMap::new();
        for window in &self.windows {
            let Some(name) = self.character_names.get(window).map(|name| name.trim()) else {
                continue;
            };
            if name.is_empty() {
                continue;
            }
            window_assignments.insert(*window, name.to_owned());
            if !profiles
                .iter()
                .any(|entry: &crate::config::CharacterConfig| entry.name == name)
            {
                profiles.push(crate::config::CharacterConfig {
                    name: name.to_owned(),
                    shortcut: self.shortcut_keys.get(window).cloned(),
                });
            }
        }
        for saved in &self.persisted_characters {
            if !saved.name.trim().is_empty()
                && !profiles.iter().any(|entry| entry.name == saved.name)
            {
                profiles.push(saved.clone());
            }
        }
        self.persisted_characters = profiles;
        // Before the first monitor snapshot, keep the assignments loaded from
        // disk; the first snapshot will prune closed windows.
        if !self.detected_windows.is_empty() {
            self.saved_window_assignments = window_assignments;
        }
    }

    pub(super) fn update_initiative_order_from_windows(&mut self) {
        let mut order = Vec::new();
        for window in &self.windows {
            let Some(name) = self
                .character_names
                .get(window)
                .map(|name| name.trim())
                .filter(|name| !name.is_empty())
            else {
                continue;
            };
            if !order.iter().any(|entry: &String| entry == name) {
                order.push(name.to_owned());
            }
        }
        for name in &self.initiative_order {
            if !order.iter().any(|entry| entry == name) {
                order.push(name.clone());
            }
        }
        self.initiative_order = order;
    }

    fn sort_windows_by_initiative_order(&mut self) {
        let order = &self.initiative_order;
        self.windows.sort_by_key(|window| {
            let name = self.character_names.get(window);
            name.and_then(|name| order.iter().position(|entry| entry == name))
                .unwrap_or(usize::MAX)
        });
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
            saved_window_assignments: HashMap::new(),
            initiative_order: Vec::new(),
            auto_order_restore_pending: false,
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
            saved_window_assignments: HashMap::new(),
            initiative_order: Vec::new(),
            auto_order_restore_pending: false,
            detected_windows: vec![1, 2],
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
            saved_window_assignments: HashMap::new(),
            initiative_order: Vec::new(),
            auto_order_restore_pending: false,
            detected_windows: Vec::new(),
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
