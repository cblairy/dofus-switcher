pub(super) fn stretch_contrast(gray: &mut [u8]) {
    let Some(&max) = gray.iter().max() else {
        return;
    };
    let min = gray
        .iter()
        .copied()
        .filter(|value| *value > 12)
        .min()
        .unwrap_or(0);
    if max <= min {
        return;
    }
    let range = u16::from(max - min);
    for value in gray {
        *value = if *value <= min {
            0
        } else {
            (u16::from(*value - min) * 255 / range).min(255) as u8
        };
    }
}
