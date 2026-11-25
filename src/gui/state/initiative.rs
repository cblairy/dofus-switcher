use x11rb::protocol::xproto::Window;

use super::WindowListState;

impl WindowListState {
    pub(crate) fn update_initiative_order_from_windows(&mut self) {
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

    pub(crate) fn sort_windows_by_initiative_order(&mut self) {
        let order = &self.initiative_order;
        self.windows.sort_by_key(|window| {
            let name = self.character_names.get(window);
            name.and_then(|name| order.iter().position(|entry| entry == name))
                .unwrap_or(usize::MAX)
        });
    }

    pub(crate) fn character_label(&self, window: Window) -> &str {
        self.character_names
            .get(&window)
            .filter(|name| !name.trim().is_empty())
            .map(String::as_str)
            .unwrap_or("Non assigné")
    }
}
