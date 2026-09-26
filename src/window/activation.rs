use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;
use x11rb::CURRENT_TIME;

pub(crate) fn activate(conn: &impl Connection, window: Window) -> Result<()> {
    let screen = &conn.setup().roots[0];
    let atom = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
        .reply()?
        .atom;

    conn.set_input_focus(InputFocus::PARENT, window, CURRENT_TIME)?;
    conn.configure_window(
        window,
        &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
    )?;

    let event = ClientMessageEvent::new(32, window, atom, [1, 0, 0, 0, 0]);

    conn.send_event(
        false,
        screen.root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        event,
    )?;
    conn.flush()?;
    Ok(())
}
