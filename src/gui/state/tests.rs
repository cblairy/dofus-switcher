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
        ..Default::default()
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
        ..Default::default()
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
        ..Default::default()
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
