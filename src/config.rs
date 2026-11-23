use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub(crate) struct PersistedConfig {
    #[serde(default = "default_version")]
    pub(crate) version: u32,
    #[serde(default)]
    pub(crate) characters: Vec<CharacterConfig>,
    /// Order as indices into the freshly detected window list.
    #[serde(default)]
    pub(crate) window_order: Vec<usize>,
    #[serde(default)]
    pub(crate) next_key: Option<String>,
    #[serde(default)]
    pub(crate) previous_key: Option<String>,
    #[serde(default)]
    pub(crate) restrict_prev_next_to_dofus: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct CharacterConfig {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) shortcut: Option<String>,
}

fn default_version() -> u32 {
    2
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

    fs::write(&path, serialized).with_context(|| format!("Ecriture de {}", path.display()))
}
