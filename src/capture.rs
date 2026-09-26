use anyhow::Result;
use x11rb::connection::Connection;
use x11rb::protocol::xproto::*;

pub fn capture_region(
    conn: &impl Connection,
    window: Window,
    x: i16,
    y: i16,
    w: u16,
    h: u16,
) -> Result<Vec<u8>> {
    let image = get_image(
        conn,
        ImageFormat::Z_PIXMAP,
        window,
        x,
        y,
        w,
        h,
        !0, // plane_mask
    )?
    .reply()?;
    Ok(image.data)
}
