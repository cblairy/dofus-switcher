use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

pub(crate) const CURRENT_VERSION: u32 = 6;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct PersistedConfig {
    #[serde(default = "initial_version")]
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

impl Default for PersistedConfig {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            characters: Vec::new(),
            ocr_region: None,
            window_assignments: Vec::new(),
            initiative_order: Vec::new(),
            window_order: Vec::new(),
            next_key: None,
            previous_key: None,
            restrict_prev_next_to_dofus: false,
        }
    }
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

fn initial_version() -> u32 {
    1
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
            let mut config: PersistedConfig =
                toml::from_str(&contents).with_context(|| format!("Parsing {}", path.display()))?;
            let migrated = migrate(&mut config)?;
            if migrated && let Err(error) = save(&config) {
                eprintln!("[config] Migration chargée mais sauvegarde impossible : {error:#}");
            }
            Ok(config)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(PersistedConfig::default())
        }
        Err(error) => Err(error).with_context(|| format!("Lecture de {}", path.display())),
    }
}

fn migrate(config: &mut PersistedConfig) -> Result<bool> {
    if config.version > CURRENT_VERSION {
        bail!(
            "La configuration utilise la version {} ; cette application ne prend en charge que la version {}.",
            config.version,
            CURRENT_VERSION
        );
    }

    if config.version == 0 {
        config.version = 1;
    }
    let original_version = config.version;

    // Version 2 introduced window_order; keep it so startup can restore legacy lists.
    // Version 3 introduced stable window assignments.
    if config.version < 3 {
        config.window_assignments.clear();
    }
    // Version 4 introduced the saved OCR capture region.
    if config.version < 4 {
        config.ocr_region = None;
    }
    // Version 6 stores ordering by character name instead of window index.
    if (3..6).contains(&config.version) && config.initiative_order.is_empty() {
        config.initiative_order = config
            .characters
            .iter()
            .map(|character| character.name.clone())
            .collect();
    }

    config.version = CURRENT_VERSION;
    Ok(original_version < CURRENT_VERSION)
}

pub(crate) fn save(config: &PersistedConfig) -> Result<()> {
    let path = config_path()?;

    if config.version != CURRENT_VERSION {
        bail!("Refus d’enregistrer une configuration qui n’est pas à jour.");
    }
    reject_newer_config(&path)?;

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

fn reject_newer_config(path: &std::path::Path) -> Result<()> {
    let contents = match fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error).with_context(|| format!("Lecture de {}", path.display())),
    };
    let value: toml::Value =
        toml::from_str(&contents).with_context(|| format!("Parsing {}", path.display()))?;
    let version = value
        .get("version")
        .and_then(toml::Value::as_integer)
        .unwrap_or(1);
    if version > i64::from(CURRENT_VERSION) {
        bail!(
            "Refus d’écraser la configuration version {version} avec la version {CURRENT_VERSION}."
        );
    }
    Ok(())
}

pub(crate) fn save_ocr_region(region: Option<OcrRegion>) -> Result<()> {
    let mut config = load()?;
    config.ocr_region = region;
    save(&config)
}
