use anyhow::{Context, Result};
use ocrs::{ImageSource, OcrEngine};
use x11rb::connection::Connection;
use x11rb::protocol::xproto::{ConnectionExt, ImageFormat, Window};
use x11rb::rust_connection::RustConnection;

use crate::config::OcrRegion;
use crate::window::ClientGeometry;

pub(super) fn process_capture(
    conn: &RustConnection,
    window: Window,
    region: OcrRegion,
    engine: &OcrEngine,
) -> Result<Option<String>> {
    let geometry = window_geometry(conn, window)?;
    let (x, y, width, height) = pixel_region(geometry, region);
    let image = conn
        .get_image(ImageFormat::Z_PIXMAP, window, x, y, width, height, u32::MAX)?
        .reply()?;
    let rgb = super::ximage::ximage_to_rgb(conn, window, image.depth, &image.data, width, height)?;

    // The character label is small in the game UI. Upscaling and normalizing
    // contrast makes the detector substantially more reliable at that size.
    let rgb = image::RgbImage::from_raw(width.into(), height.into(), rgb)
        .context("Format de capture X11 invalide")?;
    let scaled = image::imageops::resize(
        &rgb,
        width.saturating_mul(2).max(1).into(),
        height.saturating_mul(2).max(1).into(),
        image::imageops::FilterType::CatmullRom,
    );
    let mut gray = image::imageops::grayscale(&scaled);
    super::preprocess::stretch_contrast(gray.as_mut());
    let (scaled_width, scaled_height) = gray.dimensions();
    let input = engine.prepare_input(ImageSource::from_bytes(
        gray.as_raw(),
        (scaled_width, scaled_height),
    )?)?;
    let text = engine.get_text(&input)?.trim().to_owned();

    if text.chars().count() >= 2 {
        Ok(Some(text))
    } else {
        Ok(None)
    }
}

fn window_geometry(conn: &impl Connection, window: Window) -> Result<ClientGeometry> {
    let root = conn.setup().roots[0].root;
    let geometry = conn.get_geometry(window)?.reply()?;
    let origin = conn.translate_coordinates(window, root, 0, 0)?.reply()?;
    Ok(ClientGeometry {
        x: origin.dst_x,
        y: origin.dst_y,
        width: geometry.width,
        height: geometry.height,
    })
}

fn pixel_region(geometry: ClientGeometry, region: OcrRegion) -> (i16, i16, u16, u16) {
    let x = (region.x.clamp(0.0, 0.999) * f32::from(geometry.width)).round() as u16;
    let y = (region.y.clamp(0.0, 0.999) * f32::from(geometry.height)).round() as u16;
    let max_width = geometry.width.saturating_sub(x).max(1);
    let max_height = geometry.height.saturating_sub(y).max(1);
    let width = (region.width.clamp(0.001, 1.0) * f32::from(geometry.width))
        .round()
        .clamp(1.0, f32::from(max_width)) as u16;
    let height = (region.height.clamp(0.001, 1.0) * f32::from(geometry.height))
        .round()
        .clamp(1.0, f32::from(max_height)) as u16;
    (x as i16, y as i16, width, height)
}
