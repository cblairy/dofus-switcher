mod characters;
mod initiative;
#[cfg(test)]
mod tests;

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
    pub(super) config_error: Option<String>,
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
            config_error: None,
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
}
