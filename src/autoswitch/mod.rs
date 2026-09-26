use anyhow::{Context, Result};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use std::sync::mpsc::{Receiver, Sender};
use std::time::Duration;
use x11rb::errors::ReplyError;
use x11rb::rust_connection::RustConnection;

use crate::runtime::{WorkerEvent, WorkerKind};
use crate::window;

pub(crate) fn auto_switch(
    conn: &RustConnection,
    stop_rx: Receiver<()>,
    events_tx: Sender<WorkerEvent>,
) -> Result<()> {
    let engine = init_engine()?;
    let (x, y, w, h) = (76, 163, 208, 70);
    let mut dofus_windows = window::DofusWindowTracker::new();
    let mut last_ocr_error = None;

    loop {
        match stop_rx.recv_timeout(Duration::from_millis(300)) {
            Ok(()) | Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
        }

        match dofus_windows.refresh(conn) {
            Ok(Some(active_window)) => {
                report_error_recovered(&mut last_ocr_error, &events_tx);
                if let Err(error) = process_capture(conn, active_window, x, y, w, h, &engine) {
                    if is_window_closed_error(&error) {
                        eprintln!(
                            "[autoswitch] Active window (0x{active_window:x}) is closed or unavailable."
                        );
                    } else {
                        report_worker_error(
                            &mut last_ocr_error,
                            format!("OCR cycle failed: {error:#}"),
                            &events_tx,
                        )?;
                    }
                }
            }
            Ok(None) => report_error_recovered(&mut last_ocr_error, &events_tx),
            Err(error) => report_worker_error(
                &mut last_ocr_error,
                format!("OCR window tracking failed: {error:#}"),
                &events_tx,
            )?,
        }
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
        .context("Cache directory not found")?
        .join("ocrs");

    let detection_model = Model::load_file(cache_dir.join("text-detection.onnx"))
        .context("Failed to load text-detection.onnx")?;

    let recognition_model = Model::load_file(cache_dir.join("text-recognition.onnx"))
        .context("Failed to load text-recognition.onnx")?;

    OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })
    .context("Failed to initialize OCR engine")
}

fn process_capture(
    conn: &RustConnection,
    window: u32,
    x: i16,
    y: i16,
    w: u16,
    h: u16,
    engine: &OcrEngine,
) -> Result<()> {
    let raw = crate::capture::capture_region(conn, window, x, y, w, h)?;
    let img = ImageSource::from_bytes(&raw, (w as u32, h as u32))?;

    let input = engine.prepare_input(img)?;
    let text = engine.get_text(&input)?;
    let text = text.trim();

    if text.chars().count() >= 3 && !text.starts_with("Niveau") {
        println!("Detected text: {text}");
    }

    Ok(())
}
