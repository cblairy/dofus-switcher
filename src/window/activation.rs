use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::wrapper::ConnectionExt as _;
use x11rb::protocol::xproto::*;
use x11rb::protocol::Event;

pub(crate) fn activate(conn: &impl Connection, window: Window) -> Result<()> {
    let screen = &conn.setup().roots[0];
    let atom = conn
        .intern_atom(false, b"_NET_ACTIVE_WINDOW")?
        .reply()?
        .atom;

    let timestamp = server_timestamp(conn, screen.root)?;

    conn.set_input_focus(InputFocus::PARENT, window, timestamp)?;
    conn.configure_window(
        window,
        &ConfigureWindowAux::new().stack_mode(StackMode::ABOVE),
    )?;

    let event = ClientMessageEvent::new(32, window, atom, [1, timestamp, 0, 0, 0]);

    conn.send_event(
        false,
        screen.root,
        EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
        event,
    )?;
    conn.flush()?;
    Ok(())
}

fn server_timestamp(conn: &impl Connection, root: Window) -> Result<u32> {
    let dummy_atom = conn
        .intern_atom(false, b"_DOFUS_SWITCHER_TIMESTAMP")?
        .reply()?
        .atom;

    conn.change_window_attributes(
        root,
        &ChangeWindowAttributesAux::new().event_mask(EventMask::PROPERTY_CHANGE),
    )?;
    conn.change_property8(PropMode::APPEND, root, dummy_atom, AtomEnum::STRING, &[])?;
    conn.flush()?;

    loop {
        let event = conn.wait_for_event()?;
        if let Event::PropertyNotify(e) = event {
            if e.atom == dummy_atom {
                return Ok(e.time);
            }
        }
    }
}