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
