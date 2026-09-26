use super::{capture, window};

use anyhow::{Context, Result};
use ocrs::{ImageSource, OcrEngine, OcrEngineParams};
use rten::Model;
use std::thread;
use std::time::Duration;
use x11rb::errors::ReplyError;
use x11rb::rust_connection::RustConnection;

pub fn auto_switch(conn: &RustConnection) {
    let engine = match init_engine() {
        Ok(engine) => engine,
        Err(e) => {
            eprintln!("[autoswitch] Critical initialization failure: {:#}", e);
            return;
        }
    };

    let (x, y, w, h) = (76, 163, 208, 70); // TODO: valeur perso hardcodée, prendre l'input utilisateur
    let mut dofus_windows = window::DofusWindowTracker::new();

    loop {
        match dofus_windows.refresh(conn) {
            Ok(Some(active_window)) => {
                if let Err(e) = process_capture(conn, active_window, x, y, w, h, &engine) {
                    if is_window_closed_error(&e) {
                        eprintln!(
                            "[autoswitch] Active window (0x{:x}) is closed or no longer available.",
                            active_window
                        );
                        thread::sleep(Duration::from_secs(2));
                    } else {
                        eprintln!("[autoswitch] Error during cycle: {:#}", e);
                    }
                }
            }
            Ok(None) => eprintln!("[autoswitch] No active window found."),
            Err(e) => eprintln!("[autoswitch] Failed to get active window: {:#}", e),
        }

        thread::sleep(Duration::from_millis(300));
    }
}

/// Vérifie si l'erreur anyhow provient d'une fenêtre X11 fermée/invalide
fn is_window_closed_error(err: &anyhow::Error) -> bool {
    if let Some(reply_err) = err.downcast_ref::<ReplyError>() {
        if let ReplyError::X11Error(x11_err) = reply_err {
            // error_code 9 = BadDrawable, error_code 3 = BadWindow
            return x11_err.error_code == 9 || x11_err.error_code == 3;
        }
    }
    false
}

/// Initialise les modèles OCR une seule fois au lancement
fn init_engine() -> Result<OcrEngine> {
    let cache_dir = dirs::cache_dir()
        .context("Cache directory not found")?
        .join("ocrs");

    let detection_model = Model::load_file(cache_dir.join("text-detection.onnx"))
        .context("Failed to load text-detection.onnx")?;

    let recognition_model = Model::load_file(cache_dir.join("text-recognition.onnx"))
        .context("Failed to load text-recognition.onnx")?;

    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })?;

    Ok(engine)
}

/// Exécute un cycle de capture + OCR
fn process_capture(
    conn: &RustConnection,
    window: u32,
    x: i16,
    y: i16,
    w: u16,
    h: u16,
    engine: &OcrEngine,
) -> Result<()> {
    let raw = capture::capture_region(conn, window, x, y, w, h)?;
    let img = ImageSource::from_bytes(&raw, (w as u32, h as u32))?;

    let input = engine.prepare_input(img)?;
    let text = engine.get_text(&input)?;
    let text = text.trim();

    if text.chars().count() >= 3 && !text.starts_with("Niveau") {
        println!("Detected text: {}", text);
    }

    Ok(())
}
