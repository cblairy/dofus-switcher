// main.rs
mod capture;

use anyhow::Result;
use ocrs::{OcrEngine, OcrEngineParams, ImageSource};
use rten::Model;

fn main() -> Result<()> {
    let cache_dir = dirs::cache_dir().unwrap().join("ocrs");
    let detection_model = Model::load_file(cache_dir.join("text-detection.onnx"))?;
    let recognition_model = Model::load_file(cache_dir.join("text-recognition.onnx"))?;

    let engine = OcrEngine::new(OcrEngineParams {
        detection_model: Some(detection_model),
        recognition_model: Some(recognition_model),
        ..Default::default()
    })?;

    let (conn, _) = x11rb::connect(None)?;
    let window: u32 = 0x4c00008; // en dur pour CE test uniquement

    // Coordonnées de ta zone bannière, mesurées précédemment
    let (x, y, w, h) = (76, 163, 208, 70);

    loop {
        let raw = capture::capture_region(&conn, window, x, y, w, h)?;
        let img = ImageSource::from_bytes(&raw, (w as u32, h as u32))?;

        let text = engine.get_text(&engine.prepare_input(img)?)?;

        let text = text.trim();

        if text.chars().count() < 3 || text.starts_with("Niveau") {
            continue; // ou simplement ne pas printer / ne pas comparer
        }

        if !text.trim().is_empty() {
            println!("Texte détecté: {}", text.trim());
        }

        std::thread::sleep(std::time::Duration::from_millis(300));
    }
}