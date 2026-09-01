//! Loads Grim Dawn item/affix data (vendored from the grim_gleaner project,
//! https://github.com/kultcher/grim_gleaner, MIT licensed) and resolves a
//! save-file item's DBR record paths into human-readable stats.

use serde_json::Value;
use std::collections::HashMap;

/// A single resolved stat line, e.g. "Fire Damage" +18%.
/// Not yet consumed by the current UI (stats.rs works off flat HashMaps for
/// now); kept for a later pass that wants labeled/typed stat display.
#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize)]
pub struct StatLine {
    pub property_id: String,
    pub label: String,
    pub value: f64,
    /// true if this is a percent-based stat (affects how we display/sum it)
    pub is_percent: bool,
}

/// Catalog holds the equipment + affix data, indexed by DBR record_path for
/// O(1) lookup when resolving a save-file item.
pub struct Catalog {
    equipment_by_path: HashMap<String, Value>,
    affixes_by_path: HashMap<String, Value>,
    relics_by_path: HashMap<String, Value>,
    augments_by_path: HashMap<String, Value>,
    components_by_path: HashMap<String, Value>,
}

impl Catalog {
    pub fn load(data_dir: &std::path::Path) -> std::io::Result<Self> {
        let equipment = load_json(&data_dir.join("equipment.json"))?;
        let affixes = load_json(&data_dir.join("affixes.json"))?;
        let relics = load_json(&data_dir.join("relics.json"))?;
        let augments = load_json(&data_dir.join("augments.json"))?;
        let components = load_json(&data_dir.join("components.json"))?;

        Ok(Catalog {
            equipment_by_path: index_by_record_path(&equipment),
            affixes_by_path: index_by_record_path(&affixes),
            relics_by_path: index_by_record_path(&relics),
            augments_by_path: index_by_record_path(&augments),
            components_by_path: index_by_record_path(&components),
        })
    }

    /// Look up a base item, prefix, or suffix DBR path across all known
    /// catalogs (equipment carries the base item; affixes carry
    /// prefix/suffix stat lines; relics/augments/components are separate
    /// catalogs but use the same record_path keying).
    pub fn resolve_path(&self, record_path: &str) -> Option<&Value> {
        if record_path.is_empty() {
            return None;
        }
        self.equipment_by_path
            .get(record_path)
            .or_else(|| self.affixes_by_path.get(record_path))
            .or_else(|| self.relics_by_path.get(record_path))
            .or_else(|| self.augments_by_path.get(record_path))
            .or_else(|| self.components_by_path.get(record_path))
    }
}

fn load_json(path: &std::path::Path) -> std::io::Result<Value> {
    let text = std::fs::read_to_string(path)?;
    serde_json::from_str(&text).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
}

/// The catalog JSON files nest variants under each item; index every variant's
/// record_path back to the containing item+variant pair for lookup.
fn index_by_record_path(catalog: &Value) -> HashMap<String, Value> {
    let mut map = HashMap::new();
    let Some(items) = catalog.get("items").and_then(|v| v.as_array()) else {
        return map;
    };
    for item in items {
        let Some(variants) = item.get("variants").and_then(|v| v.as_array()) else {
            continue;
        };
        for variant in variants {
            if let Some(path) = variant.get("record_path").and_then(|v| v.as_str()) {
                // Merge item-level fields (display_name etc.) with the variant
                // so callers get one flat object.
                let mut merged = item.clone();
                if let Value::Object(ref mut m) = merged {
                    m.remove("variants");
                    if let Value::Object(variant_obj) = variant.clone() {
                        for (k, v) in variant_obj {
                            m.insert(k, v);
                        }
                    }
                }
                map.insert(path.to_string(), merged);
            }
        }
    }
    map
}
