use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;

pub struct DofusWindowTracker {
    windows: Vec<Window>,
    last_focused: Option<Window>,
}

impl DofusWindowTracker {
    pub fn new() -> Self {
        Self {
            windows: Vec::new(),
            last_focused: None,
        }
    }

    pub fn refresh(&mut self, conn: &impl Connection) -> Result<Option<Window>> {
        self.windows = find_dofus_windows(conn)?;

        if let Some(active) = active_window(conn, &self.windows)? {
            self.last_focused = Some(active);
            return Ok(Some(active));
        }

        Ok(self
            .last_focused
            .filter(|window| self.windows.contains(window)))
    }

    pub fn windows(&self) -> &[Window] {
        &self.windows
    }
}

pub fn find_dofus_windows(conn: &impl Connection) -> Result<Vec<Window>> {
    let root = conn.setup().roots[0].root;
    let client_list_atom = conn.intern_atom(false, b"_NET_CLIENT_LIST")?.reply()?.atom;
    let client_list = conn
        .get_property(false, root, client_list_atom, AtomEnum::WINDOW, 0, u32::MAX)?
        .reply()?;
    let windows: Vec<Window> = client_list
        .value32()
        .map(|values| values.collect())
        .unwrap_or_default();

    let windows = if windows.is_empty() {
        conn.query_tree(root)?.reply()?.children
    } else {
        windows
    };

    Ok(windows
        .into_iter()
        .filter(|window| has_dofus_identity(conn, *window))
        .collect())
}

fn has_dofus_identity(conn: &impl Connection, window: Window) -> bool {
    if read_window_property(conn, window, b"WM_CLASS").is_some_and(|value| is_dofus_class(&value)) {
        return true;
    }

    process_is_dofus(conn, window)
}

fn read_window_property(
    conn: &impl Connection,
    window: Window,
    property_name: &[u8],
) -> Option<Vec<u8>> {
    let atom = conn
        .intern_atom(false, property_name)
        .ok()?
        .reply()
        .ok()?
        .atom;
    let property = conn
        .get_property(false, window, atom, AtomEnum::ANY, 0, 1024)
        .ok()?
        .reply()
        .ok()?;
    Some(property.value)
}

fn read_window_property_u32(
    conn: &impl Connection,
    window: Window,
    property_name: &[u8],
) -> Option<u32> {
    let atom = conn
        .intern_atom(false, property_name)
        .ok()?
        .reply()
        .ok()?
        .atom;
    let property = conn
        .get_property(false, window, atom, AtomEnum::CARDINAL, 0, 1)
        .ok()?
        .reply()
        .ok()?;
    property.value32()?.next()
}

fn is_dofus_class(value: &[u8]) -> bool {
    value
        .split(|byte| *byte == 0)
        .filter_map(|part| std::str::from_utf8(part).ok())
        .any(is_dofus_executable_name)
}

fn process_is_dofus(conn: &impl Connection, window: Window) -> bool {
    let Some(pid) = read_window_property_u32(conn, window, b"_NET_WM_PID") else {
        return false;
    };

    let process_dir = std::path::PathBuf::from("/proc").join(pid.to_string());
    let executable = std::fs::read_link(process_dir.join("exe")).ok();
    let command_name = std::fs::read_to_string(process_dir.join("comm")).ok();

    executable
        .as_deref()
        .and_then(std::path::Path::file_name)
        .and_then(std::ffi::OsStr::to_str)
        .is_some_and(is_dofus_executable_name)
        || command_name
            .as_deref()
            .is_some_and(|name| is_dofus_executable_name(name.trim()))
}

fn is_dofus_executable_name(name: &str) -> bool {
    matches!(
        name.trim().to_ascii_lowercase().as_str(),
        "dofus" | "dofus.exe" | "dofus.x64" | "dofus.x86_64"
    )
}

#[cfg(test)]
mod tests {
    use super::{is_dofus_class, is_dofus_executable_name};

    #[test]
    fn identifies_dofus_window_class() {
        assert!(is_dofus_class(b"Dofus.x64\0Dofus.x64"));
        assert!(!is_dofus_class(b"code\0Visual Studio Code"));
        assert!(!is_dofus_class(b"workspace-dofus-switcher\0Code"));
    }

    #[test]
    fn matches_only_dofus_process_names() {
        assert!(is_dofus_executable_name("Dofus"));
        assert!(is_dofus_executable_name("dofus.exe"));
        assert!(is_dofus_executable_name("Dofus.x64"));
        assert!(!is_dofus_executable_name("code"));
        assert!(!is_dofus_executable_name("dofus-switcher"));
    }
}

pub fn active_window(conn: &impl Connection, dofus_windows: &[Window]) -> Result<Option<Window>> {
    let root = conn.setup().roots[0].root;
    let atom = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
        .reply()?
        .atom;
    let property = conn
        .get_property(false, root, atom, AtomEnum::WINDOW, 0, 1)?
        .reply()?;
    let active = property.value32().and_then(|mut values| values.next());

    if let Some(active) = active.filter(|window| *window != 0) {
        if dofus_windows.contains(&active) {
            return Ok(Some(active));
        }

        if let Some(dofus_ancestor) = find_dofus_ancestor(conn, active, dofus_windows) {
            return Ok(Some(dofus_ancestor));
        }
    }

    Ok(None)
}

fn find_dofus_ancestor(
    conn: &impl Connection,
    mut window: Window,
    dofus_windows: &[Window],
) -> Option<Window> {
    loop {
        let tree = conn.query_tree(window).ok()?.reply().ok()?;
        let parent = tree.parent;
        if dofus_windows.contains(&parent) {
            return Some(parent);
        }
        if parent == window || parent == conn.setup().roots[0].root {
            return None;
        }
        window = parent;
    }
}

pub fn activate(conn: &impl Connection, window: u32) -> Result<()> {
    let screen = &conn.setup().roots[0];
    let atom = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
        .reply()?
        .atom;

    let event = ClientMessageEvent::new(
        32,
        window,
        atom,
        [1, 0, 0, 0, 0], // source = application (1), timestamp, etc.
    );

    conn.send_event(
        false,
        screen.root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        event,
    )?;
    conn.flush()?;
    Ok(())
}
