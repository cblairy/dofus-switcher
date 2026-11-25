pub(super) fn region_from_overlay(
    geometry: crate::window::ClientGeometry,
    rect: [f32; 4],
    pixels_per_point: f32,
) -> Option<crate::config::OcrRegion> {
    let scale = pixels_per_point.max(0.1);
    let left = (rect[0] * scale - f32::from(geometry.x)).clamp(0.0, f32::from(geometry.width));
    let top = (rect[1] * scale - f32::from(geometry.y)).clamp(0.0, f32::from(geometry.height));
    let right = (rect[2] * scale - f32::from(geometry.x)).clamp(0.0, f32::from(geometry.width));
    let bottom = (rect[3] * scale - f32::from(geometry.y)).clamp(0.0, f32::from(geometry.height));
    if right - left < 8.0 || bottom - top < 8.0 {
        return None;
    }
    Some(crate::config::OcrRegion {
        x: left / f32::from(geometry.width),
        y: top / f32::from(geometry.height),
        width: (right - left) / f32::from(geometry.width),
        height: (bottom - top) / f32::from(geometry.height),
    })
}
