use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PersistedConfig {
    #[serde(default = "default_version")]
    pub(crate) version: u32,
    #[serde(default)]
    /// Saved character roster, stored in initiative order.
    pub(crate) characters: Vec<CharacterConfig>,
    #[serde(default)]
    pub(crate) ocr_region: Option<OcrRegion>,
    #[serde(default)]
    pub(crate) window_assignments: Vec<WindowAssignment>,
    #[serde(default)]
    pub(crate) initiative_order: Vec<String>,
    /// Legacy order as indices into the old discovery list (version 2 configs).
    #[serde(default)]
    pub(crate) window_order: Vec<usize>,
    #[serde(default)]
    pub(crate) next_key: Option<String>,
    #[serde(default)]
    pub(crate) previous_key: Option<String>,
    #[serde(default)]
    pub(crate) restrict_prev_next_to_dofus: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct OcrRegion {
    /// Region bounds normalized to the client area of a Dofus window.
    pub(crate) x: f32,
    pub(crate) y: f32,
    pub(crate) width: f32,
    pub(crate) height: f32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct CharacterConfig {
    /// Stable user identity; window IDs are only associated for the current session.
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) shortcut: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct WindowAssignment {
    /// X11 window ID, stable while the Dofus window remains open.
    pub(crate) window: u32,
    pub(crate) character: String,
}

fn default_version() -> u32 {
    6
}

fn config_path() -> Result<PathBuf> {
    let dir = dirs::config_dir()
        .context("Dossier de config introuvable")?
        .join("dofus-switcher");
    Ok(dir.join("config.toml"))
}

pub(crate) fn load() -> Result<PersistedConfig> {
    let path = config_path()?;

    match fs::read_to_string(&path) {
        Ok(contents) => {
            toml::from_str(&contents).with_context(|| format!("Parsing {}", path.display()))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(PersistedConfig::default())
        }
        Err(error) => Err(error).with_context(|| format!("Lecture de {}", path.display())),
    }
}

pub(crate) fn save(config: &PersistedConfig) -> Result<()> {
    let path = config_path()?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Création du dossier {}", parent.display()))?;
    }

    let serialized = toml::to_string_pretty(config).context("Serialisation de la config")?;
    let file_name = path
        .file_name()
        .context("Nom de fichier de configuration invalide")?
        .to_string_lossy();
    let temporary_path = path.with_file_name(format!("{file_name}.tmp-{}", std::process::id()));

    fs::write(&temporary_path, serialized)
        .with_context(|| format!("Ecriture de {}", temporary_path.display()))?;
    if let Err(error) = fs::rename(&temporary_path, &path) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error).with_context(|| {
            format!(
                "Remplacement atomique de {} par {}",
                path.display(),
                temporary_path.display()
            )
        });
    }
    Ok(())
}

pub(crate) fn save_ocr_region(region: Option<OcrRegion>) -> Result<()> {
    let mut config = load()?;
    config.ocr_region = region;
    save(&config)
}
