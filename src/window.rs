pub(crate) mod activation;
mod discovery;
mod focus;

use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::Window;

pub(crate) use discovery::find_dofus_windows;
pub(crate) use focus::active_window;

pub(crate) struct DofusWindowTracker {
    windows: Vec<Window>,
    last_focused: Option<Window>,
}

impl DofusWindowTracker {
    pub(crate) fn new() -> Self {
        Self {
            windows: Vec::new(),
            last_focused: None,
        }
    }

    pub(crate) fn refresh(&mut self, conn: &impl Connection) -> Result<Option<Window>> {
        self.windows = find_dofus_windows(conn)?;

        let active = active_window(conn, &self.windows)?;
        Ok(update_focused_window(
            &self.windows,
            active,
            &mut self.last_focused,
        ))
    }

    pub(crate) fn windows(&self) -> &[Window] {
        &self.windows
    }
}

fn update_focused_window(
    windows: &[Window],
    active: Option<Window>,
    last_focused: &mut Option<Window>,
) -> Option<Window> {
    if let Some(active) = active {
        *last_focused = Some(active);
        return Some(active);
    }

    *last_focused = last_focused.filter(|window| windows.contains(window));
    *last_focused
}

#[cfg(test)]
mod tests {
    use super::update_focused_window;

    #[test]
    fn keeps_last_focused_window_when_active_window_is_unavailable() {
        let windows = [10, 20];
        let mut last_focused = Some(20);

        assert_eq!(
            update_focused_window(&windows, None, &mut last_focused),
            Some(20)
        );
        assert_eq!(last_focused, Some(20));
    }

    #[test]
    fn clears_last_focused_window_when_it_is_no_longer_detected() {
        let windows = [10];
        let mut last_focused = Some(20);

        assert_eq!(
            update_focused_window(&windows, None, &mut last_focused),
            None
        );
        assert_eq!(last_focused, None);
    }

    #[test]
    fn active_dofus_window_replaces_last_focused_window() {
        let windows = [10, 20];
        let mut last_focused = Some(10);

        assert_eq!(
            update_focused_window(&windows, Some(20), &mut last_focused),
            Some(20)
        );
        assert_eq!(last_focused, Some(20));
    }
}
