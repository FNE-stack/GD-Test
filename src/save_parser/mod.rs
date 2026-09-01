//! Reads Grim Dawn character saves using the vendored `save-parser` crate
//! (ported from https://github.com/nbak/grim-save-parser, MIT licensed) and
//! extracts the 12 equipped-gear slots as raw DBR path triples for the
//! catalog to resolve.

use save_parser::util::map_to_json;
use serde_json::Value;
use std::fs::File;
use std::path::{Path, PathBuf};

/// One equipped item as read straight from the save file: base item + any
/// prefix/suffix/component/augment/relic DBR paths. Each is either a real
/// record_path string or empty if that slot on the item is unused.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawEquippedItem {
    pub slot_index: usize,
    pub base_name: String,
    pub prefix_name: String,
    pub suffix_name: String,
    pub modifier_name: String,
    pub relic_bonus: String,
    pub component_name: String,
    pub augment_name: String,
}

/// Default Grim Dawn save location on Windows.
pub fn default_save_dir() -> Option<PathBuf> {
    let docs = dirs_documents()?;
    let dir = docs.join("My Games").join("Grim Dawn").join("save").join("main");
    if dir.is_dir() {
        Some(dir)
    } else {
        None
    }
}

// Minimal stand-in for the `dirs` crate: reads the Windows "Documents" known
// folder via the USERPROFILE env var (works for the default, non-redirected
// case; a redirected/OneDrive Documents folder can be set explicitly in the
// app's settings instead).
fn dirs_documents() -> Option<PathBuf> {
    std::env::var_os("USERPROFILE").map(|home| PathBuf::from(home).join("Documents"))
}

/// Lists character folder names under the save dir (each is prefixed `_`).
pub fn list_characters(save_dir: &Path) -> std::io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in std::fs::read_dir(save_dir)? {
        let entry = entry?;
        if entry.path().is_dir() {
            if let Some(name) = entry.file_name().to_str() {
                if name.starts_with('_') {
                    names.push(name.to_string());
                }
            }
        }
    }
    names.sort();
    Ok(names)
}

/// Parses `<save_dir>/<character_name>/player.gdc` and returns the 12
/// equipped-gear slots (helm, chest, weapons, rings, etc — order matches
/// Grim Dawn's internal equipment array).
pub fn read_equipped_items(
    save_dir: &Path,
    character_name: &str,
) -> Result<Vec<RawEquippedItem>, String> {
    let player_path = save_dir.join(character_name).join("player.gdc");
    let file = File::open(&player_path)
        .map_err(|e| format!("could not open {}: {e}", player_path.display()))?;

    let json_text =
        map_to_json("character", file).map_err(|e| format!("save parse failed: {e}"))?;
    let parsed: Value =
        serde_json::from_str(&json_text).map_err(|e| format!("save JSON malformed: {e}"))?;

    let equipment = parsed
        .pointer("/inv/equipment")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "save JSON missing inv.equipment (unexpected save format)".to_string())?;

    let mut items = Vec::new();
    for (i, slot) in equipment.iter().enumerate() {
        let Some(item) = slot.get("item").filter(|v| !v.is_null()) else {
            continue; // empty slot
        };
        items.push(RawEquippedItem {
            slot_index: i,
            base_name: field(item, "base_name"),
            prefix_name: field(item, "prefix_name"),
            suffix_name: field(item, "suffix_name"),
            modifier_name: field(item, "modifier_name"),
            relic_bonus: field(item, "relic_bonus"),
            component_name: field(item, "component_name"),
            augment_name: field(item, "augment_name"),
        });
    }
    Ok(items)
}

fn field(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}
