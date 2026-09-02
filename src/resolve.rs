//! Glue between a raw save-file item (DBR path strings) and the catalog:
//! resolves each part into stats and merges them into one flat stat map
//! representing the fully-equipped item (base + prefix + suffix + relic +
//! augment + component all contribute their own property lines).

use crate::catalog::Catalog;
use crate::save_parser::RawEquippedItem;
use crate::stats::extract_stats;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Serialize)]
pub struct ResolvedItem {
    pub slot_index: usize,
    pub display_name: String,
    pub base_record_path: String,
    pub stats: HashMap<String, f64>,
    /// true if the base item itself wasn't found in the vendored catalog
    /// (can happen for very new/unique items not covered by the catalog
    /// scope — shown as a warning in the UI rather than silently dropped)
    pub unresolved: bool,
}

pub fn resolve_item(catalog: &Catalog, raw: &RawEquippedItem) -> ResolvedItem {
    let mut stats = HashMap::new();
    let mut display_name = String::new();
    let mut unresolved = false;

    let parts: [&str; 6] = [
        &raw.base_name,
        &raw.prefix_name,
        &raw.suffix_name,
        &raw.relic_bonus,
        &raw.component_name,
        &raw.augment_name,
    ];

    for (i, path) in parts.iter().enumerate() {
        if path.is_empty() {
            continue;
        }
        match catalog.resolve_path(path) {
            Some(resolved) => {
                if i == 0 {
                    display_name = resolved
                        .get("display_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(path)
                        .to_string();
                }
                merge_stats(&mut stats, &extract_stats(resolved));
            }
            None => {
                if i == 0 {
                    unresolved = true;
                    display_name = path.to_string();
                }
            }
        }
    }

    ResolvedItem {
        slot_index: raw.slot_index,
        display_name,
        base_record_path: raw.base_name.clone(),
        stats,
        unresolved,
    }
}

fn merge_stats(into: &mut HashMap<String, f64>, from: &HashMap<String, f64>) {
    for (k, v) in from {
        *into.entry(k.clone()).or_insert(0.0) += v;
    }
}

/// Sums stats across a full set of equipped items (baseline gear), used to
/// compute the character's current total resistances/survivability.
pub fn sum_all(items: &[ResolvedItem]) -> HashMap<String, f64> {
    let mut totals = HashMap::new();
    for item in items {
        merge_stats(&mut totals, &item.stats);
    }
    totals
}

#[allow(dead_code)]
pub fn debug_dump(v: &Value) -> String {
    serde_json::to_string_pretty(v).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::Catalog;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    /// Builds a throwaway Catalog from small fixture JSON in a unique temp
    /// dir (Catalog::load only knows how to read from disk, and its index
    /// maps are private, so an on-disk fixture is the straightforward way
    /// to exercise real cross-file resolution — base item from equipment,
    /// prefix from affixes, relic bonus/augment/component from their own
    /// catalogs — the way `resolve_item` actually uses it).
    fn build_test_catalog() -> Catalog {
        let dir = std::env::temp_dir().join(format!(
            "gd_gear_compare_test_{}_{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        std::fs::create_dir_all(&dir).unwrap();

        let equipment = serde_json::json!({
            "items": [{
                "display_name": "Test Ring",
                "variants": [{
                    "record_path": "records/items/test_ring.dbr",
                    "properties": [
                        { "property_id": "fire_resistance", "attributes": { "percent": "20.000000" } }
                    ]
                }]
            }]
        });
        let affixes = serde_json::json!({
            "affixes": [{
                "display_name": "of Embers",
                "tiers": [{
                    "record_path": "records/affixes/of_embers.dbr",
                    "properties": [
                        { "property_id": "fire_damage_percent", "attributes": { "damage_percent": "15.000000" } }
                    ]
                }]
            }]
        });
        let relics = serde_json::json!({
            "items": [{
                "display_name": "Test Relic",
                "variants": [{
                    "record_path": "records/relics/test_relic.dbr",
                    "properties": [
                        { "property_id": "offensive_ability", "attributes": { "value": "50.000000" } }
                    ]
                }]
            }]
        });
        let empty_items = serde_json::json!({ "items": [] });

        std::fs::write(
            dir.join("equipment.json"),
            serde_json::to_vec(&equipment).unwrap(),
        )
        .unwrap();
        std::fs::write(dir.join("affixes.json"), serde_json::to_vec(&affixes).unwrap()).unwrap();
        std::fs::write(dir.join("relics.json"), serde_json::to_vec(&relics).unwrap()).unwrap();
        std::fs::write(
            dir.join("augments.json"),
            serde_json::to_vec(&empty_items).unwrap(),
        )
        .unwrap();
        std::fs::write(
            dir.join("components.json"),
            serde_json::to_vec(&empty_items).unwrap(),
        )
        .unwrap();

        Catalog::load(&dir).expect("test catalog loads")
    }

    fn raw_item(base: &str, prefix: &str, relic: &str) -> crate::save_parser::RawEquippedItem {
        crate::save_parser::RawEquippedItem {
            slot_index: 0,
            base_name: base.to_string(),
            prefix_name: prefix.to_string(),
            suffix_name: String::new(),
            modifier_name: String::new(),
            relic_bonus: relic.to_string(),
            component_name: String::new(),
            augment_name: String::new(),
        }
    }

    #[test]
    fn resolves_and_merges_stats_across_base_prefix_and_relic() {
        let catalog = build_test_catalog();
        let raw = raw_item(
            "records/items/test_ring.dbr",
            "records/affixes/of_embers.dbr",
            "records/relics/test_relic.dbr",
        );
        let resolved = resolve_item(&catalog, &raw);

        assert!(!resolved.unresolved);
        assert_eq!(resolved.display_name, "Test Ring");
        assert_eq!(resolved.stats.get("fire_resistance"), Some(&20.0));
        assert_eq!(resolved.stats.get("fire_damage_percent"), Some(&15.0));
        assert_eq!(resolved.stats.get("offensive_ability"), Some(&50.0));
    }

    #[test]
    fn unknown_base_item_is_flagged_unresolved_not_dropped() {
        let catalog = build_test_catalog();
        let raw = raw_item("records/items/does_not_exist.dbr", "", "");
        let resolved = resolve_item(&catalog, &raw);

        assert!(resolved.unresolved);
        assert_eq!(resolved.display_name, "records/items/does_not_exist.dbr");
        assert!(resolved.stats.is_empty());
    }

    #[test]
    fn empty_part_paths_are_skipped_without_error() {
        let catalog = build_test_catalog();
        let raw = raw_item("records/items/test_ring.dbr", "", "");
        let resolved = resolve_item(&catalog, &raw);

        assert!(!resolved.unresolved);
        assert_eq!(resolved.stats.get("fire_resistance"), Some(&20.0));
        assert_eq!(resolved.stats.get("fire_damage_percent"), None);
    }

    #[test]
    fn sum_all_totals_stats_across_the_whole_loadout() {
        let catalog = build_test_catalog();
        let ring_a = resolve_item(
            &catalog,
            &raw_item("records/items/test_ring.dbr", "", ""),
        );
        let ring_b = resolve_item(
            &catalog,
            &raw_item("records/items/test_ring.dbr", "", ""),
        );
        let totals = sum_all(&[ring_a, ring_b]);
        assert_eq!(totals.get("fire_resistance"), Some(&40.0));
    }
}
