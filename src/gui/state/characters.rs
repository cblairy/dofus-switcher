use std::collections::HashMap;

use x11rb::protocol::xproto::Window;

use super::WindowListState;

impl WindowListState {
    pub(crate) fn assign_shortcut(&mut self, window: Window, key: Option<String>) {
        self.shortcut_keys.retain(|mapped_window, mapped_key| {
            *mapped_window != window && Some(mapped_key.clone()) != key
        });
        if let Some(key) = key {
            self.shortcut_keys.insert(window, key);
        }
    }

    pub(crate) fn assign_character(&mut self, window: Window, name: String) {
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

    pub(crate) fn create_character(&mut self, window: Window, name: String) {
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

    pub(crate) fn delete_character(&mut self, name: &str) {
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

    pub(crate) fn sync_character_profiles(&mut self) {
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
}
