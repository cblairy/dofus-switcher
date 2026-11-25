use anyhow::{Context, Result};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, Window};

#[derive(Clone, Copy, Debug)]
pub(crate) struct ClientGeometry {
    pub(crate) x: i16,
    pub(crate) y: i16,
    pub(crate) width: u16,
    pub(crate) height: u16,
}

pub(crate) fn client_geometry(window: Window) -> Result<ClientGeometry> {
    let (connection, screen_number) = x11rb::connect(None).context("Connexion à X11 impossible")?;
    let root = connection.setup().roots[screen_number].root;
    let geometry = connection
        .get_geometry(window)
        .context("Lecture de la taille de la fenêtre impossible")?
        .reply()
        .context("Fenêtre Dofus indisponible")?;
    let origin = connection
        .translate_coordinates(window, root, 0, 0)
        .context("Lecture de la position de la fenêtre impossible")?
        .reply()
        .context("Position de la fenêtre indisponible")?;

    Ok(ClientGeometry {
        x: origin.dst_x,
        y: origin.dst_y,
        width: geometry.width,
        height: geometry.height,
    })
}
