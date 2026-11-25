use anyhow::{Context, Result, bail};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::Duration;
use x11rb::connection::Connection;
use x11rb::errors::ReplyError;
use x11rb::protocol::xproto::{ConnectionExt, ImageFormat, ImageOrder, Window};
use x11rb::rust_connection::RustConnection;

use crate::config::OcrRegion;
use crate::runtime::{WorkerEvent, WorkerKind};
use crate::window::{self, ClientGeometry};

const OCR_INTERVAL: Duration = Duration::from_millis(250);

pub(crate) fn auto_switch(
    conn: &RustConnection,
    stop_rx: Receiver<()>,
    settings_rx: Receiver<Option<OcrRegion>>,
    events_tx: Sender<WorkerEvent>,
) -> Result<()> {
    let mut engine = None;
    let mut region = None;
    let mut dofus_windows = window::DofusWindowTracker::new();
    let mut last_ocr_error = None;

    loop {
        if stop_rx.try_recv().is_ok() {
            return Ok(());
        }
        loop {
            match settings_rx.try_recv() {
                Ok(next_region) => region = next_region,
                Err(TryRecvError::Empty | TryRecvError::Disconnected) => break,
            }
        }

        let Some(region) = region else {
            report_error_recovered(&mut last_ocr_error, &events_tx);
            std::thread::sleep(Duration::from_millis(100));
            continue;
        };
        if engine.is_none() {
            match init_engine() {
                Ok(ready) => {
                    engine = Some(ready);
                    report_error_recovered(&mut last_ocr_error, &events_tx);
                }
                Err(error) => {
                    report_worker_error(
                        &mut last_ocr_error,
                        format!("Initialisation OCR impossible : {error:#}"),
                        &events_tx,
                    )?;
                    std::thread::sleep(Duration::from_secs(2));
                    continue;
                }
            }
        }

        match dofus_windows.refresh(conn) {
            Ok(Some(active_window)) => {
                match process_capture(conn, active_window, region, engine.as_ref().unwrap()) {
                    Ok(detected) => {
                        report_error_recovered(&mut last_ocr_error, &events_tx);
                        let _ = events_tx.send(WorkerEvent::OcrText(detected.unwrap_or_default()));
                    }
                    Err(error) => {
                        if is_window_closed_error(&error) {
                            report_error_recovered(&mut last_ocr_error, &events_tx);
                            let _ = events_tx.send(WorkerEvent::OcrText(String::new()));
                        } else {
                            report_worker_error(
                                &mut last_ocr_error,
                                format!("Lecture OCR impossible : {error:#}"),
                                &events_tx,
                            )?;
                        }
                    }
                }
            }
            Ok(None) => {
                report_error_recovered(&mut last_ocr_error, &events_tx);
                let _ = events_tx.send(WorkerEvent::OcrText(String::new()));
            }
            Err(error) => report_worker_error(
                &mut last_ocr_error,
                format!("Suivi des fenêtres OCR impossible : {error:#}"),
                &events_tx,
            )?,
        }

        std::thread::sleep(OCR_INTERVAL);
    }
}

fn report_worker_error(
    last_error: &mut Option<String>,
    message: String,
    events_tx: &Sender<WorkerEvent>,
) -> Result<()> {
    if last_error.as_deref() != Some(message.as_str()) {
        events_tx
            .send(WorkerEvent::Error {
                worker: WorkerKind::Ocr,
                message: message.clone(),
            })
            .map_err(|_| anyhow::anyhow!("GUI worker event channel is closed"))?;
        *last_error = Some(message);
    }
    Ok(())
}

fn report_error_recovered(last_error: &mut Option<String>, events_tx: &Sender<WorkerEvent>) {
    if last_error.take().is_some() {
        let _ = events_tx.send(WorkerEvent::Recovered(WorkerKind::Ocr));
    }
}

fn is_window_closed_error(err: &anyhow::Error) -> bool {
    matches!(
        err.downcast_ref::<ReplyError>(),
        Some(ReplyError::X11Error(x11_err)) if x11_err.error_code == 9 || x11_err.error_code == 3
    )
}

fn init_engine() -> Result<OcrEngine> {
    let cache_dir = dirs::cache_dir()
        .context("Dossier de cache introuvable")?
        .join("ocrs");
    let detection_path = cache_dir.join("text-detection.onnx");
    let recognition_path = cache_dir.join("text-recognition.onnx");
    if !detection_path.is_file() || !recognition_path.is_file() {
        bail!(
            "Modèles OCR absents. Télécharge text-detection.onnx et text-recognition.onnx depuis https://github.com/robertknight/ocrs-models puis place-les dans {}.",
            cache_dir.display()
        );
    }

    let detection_model = Model::load_file(&detection_path)
        .context("Chargement du modèle de détection impossible")?;
    let recognition_model = Model::load_file(&recognition_path)
        .context("Chargement du modèle de lecture impossible")?;

    OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })
    .context("Initialisation du moteur OCR impossible")
}

fn process_capture(
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
    let rgb = ximage_to_rgb(conn, window, image.depth, &image.data, width, height)?;

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
    stretch_contrast(gray.as_mut());
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

fn ximage_to_rgb(
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

fn stretch_contrast(gray: &mut [u8]) {
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
