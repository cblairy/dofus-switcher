use anyhow::{Context, Result, bail};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, ImageOrder, Window};

pub(super) fn ximage_to_rgb(
    conn: &impl Connection,
    window: Window,
    depth: u8,
    data: &[u8],
    width: u16,
    height: u16,
) -> Result<Vec<u8>> {
    let attributes = conn.get_window_attributes(window)?.reply()?;
    let visual = conn
        .setup()
        .roots
        .iter()
        .flat_map(|screen| &screen.allowed_depths)
        .flat_map(|depth| &depth.visuals)
        .find(|visual| visual.visual_id == attributes.visual)
        .context("Format de couleurs X11 introuvable")?;
    let format = conn
        .setup()
        .pixmap_formats
        .iter()
        .find(|format| format.depth == depth)
        .context("Format de capture X11 introuvable")?;
    let bytes_per_pixel = usize::from(format.bits_per_pixel).div_ceil(8);
    let scanline_pad = usize::from(format.scanline_pad).max(8);
    let row_bytes = (usize::from(width) * usize::from(format.bits_per_pixel))
        .div_ceil(scanline_pad)
        * (scanline_pad / 8);
    let mut rgb = Vec::with_capacity(usize::from(width) * usize::from(height) * 3);

    for y in 0..usize::from(height) {
        for x in 0..usize::from(width) {
            let offset = y * row_bytes + x * bytes_per_pixel;
            let pixel_bytes = data
                .get(offset..offset + bytes_per_pixel)
                .context("Capture X11 tronquée")?;
            let pixel = match conn.setup().image_byte_order {
                ImageOrder::LSB_FIRST => pixel_bytes
                    .iter()
                    .enumerate()
                    .fold(0u32, |value, (index, byte)| {
                        value | (u32::from(*byte) << (index * 8))
                    }),
                ImageOrder::MSB_FIRST => pixel_bytes
                    .iter()
                    .fold(0u32, |value, byte| (value << 8) | u32::from(*byte)),
                _ => bail!("Ordre des pixels X11 non pris en charge"),
            };
            rgb.push(scale_channel(pixel, visual.red_mask));
            rgb.push(scale_channel(pixel, visual.green_mask));
            rgb.push(scale_channel(pixel, visual.blue_mask));
        }
    }
    Ok(rgb)
}

fn scale_channel(pixel: u32, mask: u32) -> u8 {
    if mask == 0 {
        return 0;
    }
    let shift = mask.trailing_zeros();
    let max = mask >> shift;
    let value = (pixel & mask) >> shift;
    ((value * 255 + max / 2) / max) as u8
}
