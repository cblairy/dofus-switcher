use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use anyhow::Result;

pub fn activate(conn: &impl Connection, window: u32) -> Result<()> {
    let screen = &conn.setup().roots[0];
    let atom = conn.intern_atom(false, b"_NET_ACTIVE_WINDOW")?.reply()?.atom;

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