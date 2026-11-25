use std::collections::HashSet;

use super::WindowListApp;

impl WindowListApp {
    pub(super) fn persisted_config(&self) -> crate::config::PersistedConfig {
        let mut window_assignments: Vec<_> = self
            .state
            .saved_window_assignments
            .iter()
            .map(|(window, character)| crate::config::WindowAssignment {
                window: *window,
                character: character.clone(),
            })
            .collect();
        window_assignments.sort_by_key(|assignment| assignment.window);

        crate::config::PersistedConfig {
            version: crate::config::CURRENT_VERSION,
            characters: self.state.persisted_characters.clone(),
            ocr_region: self.state.ocr_region,
            window_assignments,
            initiative_order: self.state.initiative_order.clone(),
            window_order: Vec::new(),
            next_key: self.state.next_key.clone(),
            previous_key: self.state.previous_key.clone(),
            restrict_prev_next_to_dofus: self.state.restrict_prev_next_to_dofus,
        }
    }

    pub(super) fn persist_config(&mut self) {
        let config = self.persisted_config();
        if self.last_saved_config.as_ref() == Some(&config) {
            return;
        }

        match crate::config::save(&config) {
            Ok(()) => {
                self.last_saved_config = Some(config);
                self.state.config_error = None;
            }
            Err(error) => {
                self.state.config_error = Some(format!("Sauvegarde impossible : {error:#}"));
            }
        }
    }
}

pub(super) fn discard_incremental_name_fragments(
    characters: Vec<crate::config::CharacterConfig>,
) -> Vec<crate::config::CharacterConfig> {
    let mut fragments = HashSet::new();
    for complete_name in characters
        .iter()
        .filter(|character| character.shortcut.is_none())
    {
        let complete = complete_name.name.to_lowercase();
        let prefixes: Vec<_> = characters
            .iter()
            .filter(|character| {
                character.shortcut.is_none()
                    && character.name.chars().count() >= 2
                    && character.name.chars().count() < complete_name.name.chars().count()
                    && complete.starts_with(&character.name.to_lowercase())
            })
            .collect();
        // Older builds saved every intermediate value while the user typed.
        // Only remove a clear chain of at least three such prefixes; retain
        // shorter legitimate character names and anything with a shortcut.
        if prefixes.len() >= 3 {
            fragments.extend(prefixes.into_iter().map(|character| character.name.clone()));
        }
    }
    characters
        .into_iter()
        .filter(|character| !fragments.contains(&character.name))
        .collect()
}
