//! Stat extraction and comparison logic: turns a resolved catalog item
//! (JSON `properties` array) into a flat stat map, and compares two items
//! against a set of user-defined priority weights plus resistance/survival
//! awareness.

use serde_json::Value;
use std::collections::HashMap;

/// Stats where a "maximum_X_resistance" property raises the cap rather than
/// filling toward it. We track these separately so the comparator can warn
/// "this pushes you over your current cap" vs "this is wasted, you're already capped".
pub const RESISTANCE_STATS: &[&str] = &[
    "fire_resistance",
    "cold_resistance",
    "lightning_resistance",
    "aether_resistance",
    "chaos_resistance",
    "vitality_resistance",
    "poison_acid_resistance",
    "pierce_resistance",
    "physical_resistance",
    "bleeding_resistance",
    "elemental_resistance", // shorthand that applies to fire+cold+lightning
];

/// Not yet wired into the compare UI (that only shows resistance impact
/// today) — reserved for a survivability panel in a later pass.
#[allow(dead_code)]
pub const SURVIVAL_STATS: &[&str] = &[
    "armor",
    "armor_percent",
    "total_health",
    "flat_health_regen",
    "percent_health_regen",
    "defensive_ability",
    "physique",
    "flat_life_leech_percent",
];

/// Default resistance cap in Grim Dawn (before augment/skill-based increases).
pub const DEFAULT_RESIST_CAP: f64 = 80.0;

/// Flat map of property_id -> summed numeric value for one item.
/// Percent and flat variants of the same property_id are summed together
/// (grim_gleaner's catalog already separates e.g. "armor" vs "armor_percent"
/// as distinct property_ids, so no unit-mixing here).
pub fn extract_stats(resolved: &Value) -> HashMap<String, f64> {
    let mut stats = HashMap::new();
    let Some(properties) = resolved.get("properties").and_then(|v| v.as_array()) else {
        return stats;
    };
    for prop in properties {
        let Some(id) = prop.get("property_id").and_then(|v| v.as_str()) else {
            continue;
        };
        let Some(attrs) = prop.get("attributes").and_then(|v| v.as_object()) else {
            continue;
        };
        // attributes can be "value", "percent", or min/max damage pairs.
        // For scoring we take the most representative single number:
        // prefer "value"/"percent", else average of min/max.
        let value = if let Some(v) = attrs.get("value").and_then(parse_num) {
            v
        } else if let Some(v) = attrs.get("percent").and_then(parse_num) {
            v
        } else {
            let min = attrs
                .iter()
                .find(|(k, _)| k.ends_with("_min"))
                .and_then(|(_, v)| parse_num(v));
            let max = attrs
                .iter()
                .find(|(k, _)| k.ends_with("_max"))
                .and_then(|(_, v)| parse_num(v));
            match (min, max) {
                (Some(a), Some(b)) => (a + b) / 2.0,
                (Some(a), None) => a,
                (None, Some(b)) => b,
                (None, None) => continue,
            }
        };
        *stats.entry(id.to_string()).or_insert(0.0) += value;
    }
    stats
}

fn parse_num(v: &Value) -> Option<f64> {
    v.as_str()?.parse::<f64>().ok()
}

/// User-defined priority weight for one stat, 0-4 stars (matches grim_gleaner's
/// weighting concept). 0 = ignored, 4 = top priority.
pub type PrioWeights = HashMap<String, u8>;

/// Weighted-relevance score for one item against a set of priorities.
/// Mirrors grim_gleaner's "relevance of stats, not their values" approach:
/// having ANY amount of a 4-star stat counts heavily; the actual roll only
/// matters as a secondary tiebreaker within the same relevance tier.
pub fn prio_score(stats: &HashMap<String, f64>, weights: &PrioWeights) -> f64 {
    let mut score = 0.0;
    for (stat, weight) in weights {
        if *weight == 0 {
            continue;
        }
        if let Some(&value) = stats.get(stat) {
            if value != 0.0 {
                // base relevance points from the star weight, small bonus
                // from magnitude so bigger rolls still edge out smaller ones
                score += (*weight as f64) * 10.0 + value.abs().min(50.0) * 0.1;
            }
        }
    }
    score
}

/// Letter grade matching grim_gleaner's F..S++ scale, based on how much of
/// the theoretical max weighted score this item hits.
pub fn letter_grade(score: f64, max_possible: f64) -> &'static str {
    if max_possible <= 0.0 {
        return "-";
    }
    let pct = (score / max_possible * 100.0).clamp(0.0, 200.0);
    match pct {
        p if p >= 130.0 => "S++",
        p if p >= 110.0 => "S+",
        p if p >= 95.0 => "S",
        p if p >= 80.0 => "A",
        p if p >= 65.0 => "B",
        p if p >= 50.0 => "C",
        p if p >= 30.0 => "D",
        _ => "F",
    }
}

/// The theoretical max score if an item had every weighted stat at a
/// reasonably strong roll (~30, tuned against typical GD affix magnitudes).
pub fn max_possible_score(weights: &PrioWeights) -> f64 {
    weights
        .values()
        .filter(|&&w| w > 0)
        .map(|&w| (w as f64) * 10.0 + 3.0)
        .sum()
}

#[derive(Debug, serde::Serialize)]
pub struct ResistImpact {
    pub stat: String,
    pub current_total: f64,
    pub after_total: f64,
    pub delta: f64,
    /// true if `after_total` exceeds the resist cap (wasted overcap, unless
    /// a maximum_*_resistance stat is also present to raise the cap)
    pub over_cap: bool,
    /// true if `after_total` is at or below 0 (dangerous, taking bonus damage)
    pub dangerous: bool,
}

/// Computes resistance deltas: current baseline totals (across all equipped
/// gear) vs. totals after swapping in the candidate item for the given slot.
pub fn resist_impact(
    baseline_totals: &HashMap<String, f64>,
    removed_item_stats: &HashMap<String, f64>,
    added_item_stats: &HashMap<String, f64>,
) -> Vec<ResistImpact> {
    let mut out = Vec::new();
    for &stat in RESISTANCE_STATS {
        let current = *baseline_totals.get(stat).unwrap_or(&0.0);
        let removed = *removed_item_stats.get(stat).unwrap_or(&0.0);
        let added = *added_item_stats.get(stat).unwrap_or(&0.0);
        let after = current - removed + added;
        if current == 0.0 && after == 0.0 {
            continue;
        }
        let cap_key = format!("maximum_{stat}");
        let cap_bonus = *baseline_totals.get(&cap_key).unwrap_or(&0.0);
        let effective_cap = DEFAULT_RESIST_CAP + cap_bonus;
        out.push(ResistImpact {
            stat: stat.to_string(),
            current_total: current,
            after_total: after,
            delta: after - current,
            over_cap: after > effective_cap,
            dangerous: after <= 0.0,
        });
    }
    out
}
