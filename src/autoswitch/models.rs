use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use rten::Model;

const DETECTION_MODEL: &str = "text-detection.onnx";
const RECOGNITION_MODEL: &str = "text-recognition.onnx";

pub(super) fn load_pair() -> Result<(Model, Model)> {
    let cache_dir = dirs::cache_dir()
        .context("Dossier de cache introuvable")?
        .join("ocrs");
    let mut candidates = packaged_model_dirs();
    candidates.push(cache_dir.clone());

    let Some(model_dir) = candidates.into_iter().find(|dir| has_model_pair(dir)) else {
        bail!(
            "Modèles OCR absents. Le paquet doit contenir les modèles dans share/dofus-switcher/models ; sinon, place text-detection.onnx et text-recognition.onnx dans {}.",
            cache_dir.display()
        );
    };

    let detection = Model::load_file(model_dir.join(DETECTION_MODEL))
        .context("Chargement du modèle de détection impossible")?;
    let recognition = Model::load_file(model_dir.join(RECOGNITION_MODEL))
        .context("Chargement du modèle de lecture impossible")?;
    Ok((detection, recognition))
}

fn has_model_pair(dir: &std::path::Path) -> bool {
    dir.join(DETECTION_MODEL).is_file() && dir.join(RECOGNITION_MODEL).is_file()
}

fn packaged_model_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(app_dir) = std::env::var_os("APPDIR") {
        dirs.push(PathBuf::from(app_dir).join("usr/share/dofus-switcher/models"));
    }
    if let Ok(executable) = std::env::current_exe()
        && let Some(bin_dir) = executable.parent()
        && let Some(prefix_dir) = bin_dir.parent()
    {
        dirs.push(prefix_dir.join("share/dofus-switcher/models"));
    }
    dirs
}
