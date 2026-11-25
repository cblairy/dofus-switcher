use std::collections::HashMap;

use handy_keys::{Hotkey, HotkeyId, HotkeyManager};
use x11rb::protocol::xproto::Window;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HotkeyAction {
    Window(Window),
    Next,
    Previous,
}

pub(crate) struct HotkeyRegistry {
    window_shortcuts: HashMap<Window, Hotkey>,
    window_hotkey_ids: HashMap<Window, HotkeyId>,
    hotkey_windows: HashMap<HotkeyId, Window>,
    hotkey_actions: HashMap<HotkeyId, HotkeyAction>,
}

impl HotkeyRegistry {
    pub(crate) fn new() -> Self {
        Self {
            window_shortcuts: HashMap::new(),
            window_hotkey_ids: HashMap::new(),
            hotkey_windows: HashMap::new(),
            hotkey_actions: HashMap::new(),
        }
    }

    pub(crate) fn find_duplicate_window(&self, hotkey: &Hotkey) -> Option<Window> {
        self.window_shortcuts
            .iter()
            .find_map(|(w, h)| (h == hotkey).then_some(*w))
    }

    pub(crate) fn register_window(
        &mut self,
        manager: &HotkeyManager,
        window: Window,
        hotkey: Hotkey,
    ) -> Result<HotkeyId, String> {
        match manager.register(hotkey) {
            Ok(id) => {
                self.window_shortcuts.insert(window, hotkey);
                self.window_hotkey_ids.insert(window, id);
                self.hotkey_windows.insert(id, window);
                self.hotkey_actions.insert(id, HotkeyAction::Window(window));
                Ok(id)
            }
            Err(e) => Err(format!("{e:#}")),
        }
    }

    pub(crate) fn unregister_window(&mut self, manager: &HotkeyManager, window: Window) {
        self.window_shortcuts.remove(&window);
        if let Some(id) = self.window_hotkey_ids.remove(&window) {
            let _ = manager.unregister(id);
            self.hotkey_windows.remove(&id);
            self.hotkey_actions.remove(&id);
        }
    }

    pub(crate) fn unregister_global(&mut self, manager: &HotkeyManager, action: HotkeyAction) {
        let id = self
            .hotkey_actions
            .iter()
            .find_map(|(id, act)| (*act == action).then_some(*id));
        if let Some(id) = id {
            let _ = manager.unregister(id);
            self.hotkey_actions.remove(&id);
            self.hotkey_windows.remove(&id);
        }
    }

    pub(crate) fn register_global(
        &mut self,
        manager: &HotkeyManager,
        action: HotkeyAction,
        hotkey: Hotkey,
    ) -> Result<HotkeyId, String> {
        match manager.register(hotkey) {
            Ok(id) => {
                self.hotkey_actions.insert(id, action);
                Ok(id)
            }
            Err(e) => Err(format!("{e:#}")),
        }
    }

    pub(crate) fn get_window_for_id(&self, id: &HotkeyId) -> Option<Window> {
        self.hotkey_windows.get(id).copied()
    }

    pub(crate) fn get_action_for_id(&self, id: &HotkeyId) -> Option<HotkeyAction> {
        self.hotkey_actions.get(id).copied()
    }
}
