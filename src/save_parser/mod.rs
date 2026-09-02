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

/// One item sitting in a stash/backpack sack, as read straight from the
/// save file — same DBR-path fields as RawEquippedItem, plus the roll
/// seeds. Used for the "check for new items" feature: a plain path match
/// isn't enough to tell two *different* drops of an identical affix
/// combination apart, so the seeds are kept for identity purposes even
/// though (like everywhere else in this app) they don't affect anything
/// resolved from the catalog.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawInventoryItem {
    pub sack_index: usize,
    pub base_name: String,
    pub prefix_name: String,
    pub suffix_name: String,
    pub modifier_name: String,
    pub transmute_name: String,
    pub relic_bonus: String,
    pub component_name: String,
    pub augment_name: String,
    pub seed: u64,
    pub component_seed: u64,
    pub augment_seed: u64,
}

impl RawInventoryItem {
    /// A stable identity string for "have we seen this exact item before" —
    /// every resolvable DBR path plus every roll seed. Two items are only
    /// considered the same if all of these match; a duplicate drop of an
    /// identical affix combination (different seed) still counts as new.
    pub fn identity_key(&self) -> String {
        format!(
            "{}|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.base_name,
            self.prefix_name,
            self.suffix_name,
            self.modifier_name,
            self.transmute_name,
            self.relic_bonus,
            self.component_name,
            self.augment_name,
            self.seed,
            self.component_seed,
            self.augment_seed,
        )
    }

    /// Adapts to RawEquippedItem so it can go through the same catalog
    /// resolution path as equipped gear (`resolve::resolve_item`) — a
    /// sack item's affixes live on the same Item fields as an equipped
    /// item's, just without a meaningful equipment slot_index.
    pub fn as_equipped(&self) -> RawEquippedItem {
        RawEquippedItem {
            slot_index: self.sack_index,
            base_name: self.base_name.clone(),
            prefix_name: self.prefix_name.clone(),
            suffix_name: self.suffix_name.clone(),
            modifier_name: self.modifier_name.clone(),
            relic_bonus: self.relic_bonus.clone(),
            component_name: self.component_name.clone(),
            augment_name: self.augment_name.clone(),
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(base: &str, prefix: &str, seed: u64) -> RawInventoryItem {
        RawInventoryItem {
            sack_index: 0,
            base_name: base.to_string(),
            prefix_name: prefix.to_string(),
            suffix_name: String::new(),
            modifier_name: String::new(),
            transmute_name: String::new(),
            relic_bonus: String::new(),
            component_name: String::new(),
            augment_name: String::new(),
            seed,
            component_seed: 0,
            augment_seed: 0,
        }
    }

    #[test]
    fn identity_key_matches_for_identical_items() {
        let a = sample("records/items/gearhead/b014a_head.dbr", "records/items/lootaffixes/prefix/x.dbr", 12345);
        let b = sample("records/items/gearhead/b014a_head.dbr", "records/items/lootaffixes/prefix/x.dbr", 12345);
        assert_eq!(a.identity_key(), b.identity_key());
    }

    #[test]
    fn identity_key_differs_on_seed_alone() {
        // Two drops of the exact same base+affix combination, different
        // rolls — a real "new item" should still show up as new, not get
        // silently treated as "already seen" just because the affixes match.
        let a = sample("records/items/materia/compa_aethercrystal.dbr", "", 111);
        let b = sample("records/items/materia/compa_aethercrystal.dbr", "", 222);
        assert_ne!(a.identity_key(), b.identity_key());
    }

    #[test]
    fn identity_key_differs_on_affix_alone() {
        let a = sample("records/items/gearhead/b014a_head.dbr", "records/items/lootaffixes/prefix/x.dbr", 1);
        let b = sample("records/items/gearhead/b014a_head.dbr", "records/items/lootaffixes/prefix/y.dbr", 1);
        assert_ne!(a.identity_key(), b.identity_key());
    }

    #[test]
    fn as_equipped_carries_resolvable_fields_through_unchanged() {
        let raw = sample("records/items/gearhead/b014a_head.dbr", "records/items/lootaffixes/prefix/x.dbr", 1);
        let equipped = raw.as_equipped();
        assert_eq!(equipped.base_name, raw.base_name);
        assert_eq!(equipped.prefix_name, raw.prefix_name);
        assert_eq!(equipped.suffix_name, raw.suffix_name);
    }
}

fn field(v: &Value, key: &str) -> String {
    v.get(key).and_then(|x| x.as_str()).unwrap_or("").to_string()
}

fn int_field(v: &Value, key: &str) -> u64 {
    v.get(key).and_then(|x| x.as_u64()).unwrap_or(0)
}

/// Parses `<save_dir>/<character_name>/player.gdc` and returns every item
/// sitting in the character's stash/backpack sacks (not the shared
/// account stash — that's a separate .gst file this app doesn't read).
/// Used by the "check for new items" feature to diff against what was
/// there last time.
pub fn read_inventory_items(
    save_dir: &Path,
    character_name: &str,
) -> Result<Vec<RawInventoryItem>, String> {
    let player_path = save_dir.join(character_name).join("player.gdc");
    let file = File::open(&player_path)
        .map_err(|e| format!("could not open {}: {e}", player_path.display()))?;

    let json_text =
        map_to_json("character", file).map_err(|e| format!("save parse failed: {e}"))?;
    let parsed: Value =
        serde_json::from_str(&json_text).map_err(|e| format!("save JSON malformed: {e}"))?;

    let sacks = parsed
        .pointer("/inv/sacks")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "save JSON missing inv.sacks (unexpected save format)".to_string())?;

    let mut items = Vec::new();
    for (sack_index, sack) in sacks.iter().enumerate() {
        let Some(sack_items) = sack.get("items").and_then(|v| v.as_array()) else {
            continue;
        };
        for entry in sack_items {
            let Some(item) = entry.get("item") else {
                continue;
            };
            let base_name = field(item, "base_name");
            if base_name.is_empty() {
                continue; // defensive — every real sack entry should have one
            }
            items.push(RawInventoryItem {
                sack_index,
                base_name,
                prefix_name: field(item, "prefix_name"),
                suffix_name: field(item, "suffix_name"),
                modifier_name: field(item, "modifier_name"),
                transmute_name: field(item, "transmute_name"),
                relic_bonus: field(item, "relic_bonus"),
                component_name: field(item, "component_name"),
                augment_name: field(item, "augment_name"),
                seed: int_field(item, "seed"),
                component_seed: int_field(item, "component_seed"),
                augment_seed: int_field(item, "augment_seed"),
            });
        }
    }
    Ok(items)
}
