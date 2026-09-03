//! Reads Grim Dawn character saves using the vendored `save-parser` crate
//! (ported from https://github.com/nbak/grim-save-parser, MIT licensed) and
//! extracts equipped-gear slots (12 fixed body slots plus the active
//! weapon set) as raw DBR path triples for the catalog to resolve.

use save_parser::util::map_to_json;
use serde_json::Value;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

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

/// One item sitting somewhere other than equipped — a personal inventory
/// bag or a personal stash tab (not the shared/transfer stash, a separate
/// .gst file this app doesn't read) — as read straight from the save file.
/// Same DBR-path fields as RawEquippedItem, plus the roll seeds and a
/// human-readable `source` (e.g. "Bag 1", "Stash Tab 3") for display. Used
/// by the "items in your bags & stash" feature: a plain path match isn't
/// enough to tell two *different* drops of an identical affix combination
/// apart, so the seeds are kept for identity purposes even though (like
/// everywhere else in this app) they don't affect anything resolved from
/// the catalog. `source` is deliberately excluded from that identity —
/// moving an item from a bag into a stash tab doesn't make it "new".
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawInventoryItem {
    pub source: String,
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
    /// every resolvable DBR path plus every roll seed, deliberately *not*
    /// `source` (see the struct doc). Two items are only considered the
    /// same if all of these match; a duplicate drop of an identical affix
    /// combination (different seed) still counts as new.
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
    /// bag/stash item's affixes live on the same Item fields as an
    /// equipped item's. slot_index is meaningless here (this item isn't
    /// equipped in any slot) and isn't surfaced by anything that calls
    /// this, so it's just 0.
    pub fn as_equipped(&self) -> RawEquippedItem {
        RawEquippedItem {
            slot_index: 0,
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

/// Unix timestamp (seconds) of `player.gdc`'s last write — i.e. the last
/// time Grim Dawn actually saved, not "now". Grim Dawn only writes on
/// specific triggers (autosave, leaving an area, opening the menu, quitting
/// — not the instant an item is picked up or moved), so everything this
/// app reads can be meaningfully stale; exposing this lets the UI show
/// that plainly instead of implying the numbers are live.
pub fn save_file_mtime(save_dir: &Path, character_name: &str) -> Option<u64> {
    let player_path = save_dir.join(character_name).join("player.gdc");
    let modified = std::fs::metadata(&player_path).ok()?.modified().ok()?;
    modified
        .duration_since(SystemTime::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// Parses `<save_dir>/<character_name>/player.gdc` and returns the equipped
/// gear: the 12 fixed body slots (helm, chest, rings, etc — slot 11 is the
/// Relic slot, confirmed against a real character's data; despite the
/// name, it is not a weapon slot), plus slots 12/13 for the *currently
/// active* weapon set's main-hand and off-hand.
///
/// Weapons are not part of the 12-slot `inv.equipment` array at all — they
/// live in separate `inv.weapon1`/`inv.weapon2` arrays (each [main-hand,
/// off-hand]) for Grim Dawn's two weapon-swap sets, gated by `use_alternate`
/// (0 = set 1 active, otherwise set 2). Missing this entirely previously
/// meant "your weapon" never showed up anywhere in this app. Only the
/// active set is read — the point is comparing what's *currently*
/// equipped, and the inactive swap set isn't that.
pub fn read_equipped_items(
    save_dir: &Path,
    character_name: &str,
) -> Result<Vec<RawEquippedItem>, String> {
    let parsed = read_character_json(save_dir, character_name)?;

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

    let use_alternate = parsed
        .pointer("/inv/use_alternate")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    let active_weapon_set = if use_alternate == 0 {
        "/inv/weapon1"
    } else {
        "/inv/weapon2"
    };
    if let Some(weapons) = parsed.pointer(active_weapon_set).and_then(|v| v.as_array()) {
        // WEAPON_SLOT_START matches app.js's SLOT_LABELS (12 = Main Hand,
        // 13 = Off-Hand) — keep the two in sync if this ever changes.
        const WEAPON_SLOT_START: usize = 12;
        for (i, slot) in weapons.iter().enumerate() {
            let Some(item) = slot.get("item").filter(|v| !v.is_null()) else {
                continue;
            };
            let base_name = field(item, "base_name");
            if base_name.is_empty() {
                continue; // e.g. off-hand slot with a 2-handed weapon equipped
            }
            items.push(RawEquippedItem {
                slot_index: WEAPON_SLOT_START + i,
                base_name,
                prefix_name: field(item, "prefix_name"),
                suffix_name: field(item, "suffix_name"),
                modifier_name: field(item, "modifier_name"),
                relic_bonus: field(item, "relic_bonus"),
                component_name: field(item, "component_name"),
                augment_name: field(item, "augment_name"),
            });
        }
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(base: &str, prefix: &str, seed: u64) -> RawInventoryItem {
        RawInventoryItem {
            source: "Bag 1".to_string(),
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
    fn identity_key_ignores_source_alone() {
        // Moving an item from a bag into a stash tab (or vice versa) is
        // the same item, not a new one — it shouldn't reset its "seen"
        // status just because it changed location.
        let mut in_bag = sample("records/items/gearhead/b014a_head.dbr", "", 1);
        in_bag.source = "Bag 1".to_string();
        let mut in_stash = in_bag.clone();
        in_stash.source = "Stash Tab 3".to_string();
        assert_eq!(in_bag.identity_key(), in_stash.identity_key());
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
/// sitting in the character's personal inventory bags (not the shared
/// account/transfer stash — that's a separate .gst file this app doesn't
/// read; see `read_stash_items` for the *personal* stash, which is a
/// different thing and lives right in this same file). Used by the "items
/// in your bags & stash" feature to list everything comparable, diffed
/// against what was there last time to flag what's new.
pub fn read_inventory_items(
    save_dir: &Path,
    character_name: &str,
) -> Result<Vec<RawInventoryItem>, String> {
    let parsed = read_character_json(save_dir, character_name)?;

    let sacks = parsed
        .pointer("/inv/sacks")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "save JSON missing inv.sacks (unexpected save format)".to_string())?;

    let mut items = Vec::new();
    for (sack_index, sack) in sacks.iter().enumerate() {
        let Some(sack_items) = sack.get("items").and_then(|v| v.as_array()) else {
            continue;
        };
        let source = format!("Bag {}", sack_index + 1);
        for entry in sack_items {
            let Some(item) = entry.get("item") else {
                continue;
            };
            let base_name = field(item, "base_name");
            if base_name.is_empty() {
                continue; // defensive — every real sack entry should have one
            }
            items.push(RawInventoryItem {
                source: source.clone(),
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

/// Parses `<save_dir>/<character_name>/player.gdc` and returns every item
/// sitting in the character's *personal* stash tabs — the in-town stash
/// this character alone can access, stored right in this same save file
/// (`/stash/tabs`, a sibling of `/inv`, not nested under it). Not the
/// shared/transfer stash used to move items between characters — that's
/// account-wide and lives in a separate .gst file this app doesn't read.
/// Reads however many tabs the character has actually unlocked, not a
/// fixed count.
pub fn read_stash_items(
    save_dir: &Path,
    character_name: &str,
) -> Result<Vec<RawInventoryItem>, String> {
    let parsed = read_character_json(save_dir, character_name)?;

    let tabs = parsed
        .pointer("/stash/tabs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "save JSON missing stash.tabs (unexpected save format)".to_string())?;

    let mut items = Vec::new();
    for (tab_index, tab) in tabs.iter().enumerate() {
        let Some(tab_items) = tab.get("items").and_then(|v| v.as_array()) else {
            continue;
        };
        let source = format!("Stash Tab {}", tab_index + 1);
        for entry in tab_items {
            let Some(item) = entry.get("item") else {
                continue;
            };
            let base_name = field(item, "base_name");
            if base_name.is_empty() {
                continue;
            }
            items.push(RawInventoryItem {
                source: source.clone(),
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

/// Shared by read_inventory_items/read_stash_items/read_equipped_items:
/// opens and fully parses one character's player.gdc into JSON.
fn read_character_json(save_dir: &Path, character_name: &str) -> Result<Value, String> {
    let player_path = save_dir.join(character_name).join("player.gdc");
    let file = File::open(&player_path)
        .map_err(|e| format!("could not open {}: {e}", player_path.display()))?;
    let json_text =
        map_to_json("character", file).map_err(|e| format!("save parse failed: {e}"))?;
    serde_json::from_str(&json_text).map_err(|e| format!("save JSON malformed: {e}"))
}
