use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;

pub(crate) fn active_window(
    conn: &impl Connection,
    dofus_windows: &[Window],
) -> Result<Option<Window>> {
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
