//! Small persisted app config (settings.json next to the exe) — currently
//! just an optional override for the Grim Dawn save folder, for players
//! whose save isn't in the default Documents\My Games\Grim Dawn\save\main
//! location (custom Documents redirect, save on another drive, etc).

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    /// User-chosen override for the save folder. When None, the app falls
    /// back to auto-detection (see save_parser::default_save_dir).
    pub save_dir_override: Option<PathBuf>,
}

impl Settings {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let text = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        std::fs::write(path, text)
    }
}

/// Validates that `path` looks like a real Grim Dawn save folder: it must
/// exist, be a directory, and contain at least one character subfolder
/// (Grim Dawn names these with a leading underscore, e.g. "_MyCharacter").
pub fn validate_save_dir(path: &Path) -> Result<(), String> {
    if !path.is_dir() {
        return Err(format!("{} is not a folder", path.display()));
    }
    let has_character = std::fs::read_dir(path)
        .map_err(|e| format!("could not read {}: {e}", path.display()))?
        .filter_map(|e| e.ok())
        .any(|e| {
            e.path().is_dir()
                && e.file_name().to_str().is_some_and(|n| n.starts_with('_'))
        });
    if !has_character {
        return Err(format!(
            "{} doesn't look like a Grim Dawn save folder (no character subfolders found — \
             expected something like \"_YourCharacterName\" inside it)",
            path.display()
        ));
    }
    Ok(())
}
