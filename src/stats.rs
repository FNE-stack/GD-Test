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

/// Attribute keys observed across grim_gleaner's whole catalog (equipment,
/// affixes, relics, augments, components — verified by scanning all five
/// files) that represent a single core magnitude rather than a qualifier.
/// "value"/"percent"/"flat"/"damage_percent" all mean "the number" for their
/// property (just from different source DBR fields), so they collapse onto
/// the bare property_id — this matters a lot for "damage_percent": it's the
/// sole attribute on ~13k properties in equipment.json alone (every
/// "X_damage_percent" stat, e.g. fire_damage_percent, aether_damage_percent),
/// so missing it here previously meant those extremely common offense stats
/// were silently stored as "fire_damage_percent:damage_percent" instead of
/// the bare "fire_damage_percent" the priority taxonomy weights — they never
/// matched a user's set priority, so items with big elemental % rolls scored
/// as if they had nothing. Every other numeric key (chance_percent,
/// reduction_flat, skill_level, duration_*, component, ...) is kept but
/// suffixed onto the property_id so it isn't silently dropped or wrongly
/// merged into an unrelated number.
const PRIMARY_MAGNITUDE_KEYS: &[&str] = &["value", "percent", "flat", "damage_percent"];

/// Flat map of stat_key -> summed numeric value for one item, where
/// stat_key is usually the bare property_id (for the primary magnitude)
/// and `property_id:attribute_key` for secondary attributes (chance,
/// duration, reduction, skill_level, etc).
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

        // Primary magnitude: prefer value/percent/flat directly; otherwise
        // average a *_min/*_max pair (e.g. damage_min/damage_max,
        // percent_min/percent_max) into the bare property_id.
        let primary = PRIMARY_MAGNITUDE_KEYS
            .iter()
            .find_map(|k| attrs.get(*k).and_then(parse_num))
            .or_else(|| {
                let min = attrs
                    .iter()
                    .find(|(k, _)| k.ends_with("_min"))
                    .and_then(|(_, v)| parse_num(v));
                let max = attrs
                    .iter()
                    .find(|(k, _)| k.ends_with("_max"))
                    .and_then(|(_, v)| parse_num(v));
                match (min, max) {
                    (Some(a), Some(b)) => Some((a + b) / 2.0),
                    (Some(a), None) => Some(a),
                    (None, Some(b)) => Some(b),
                    (None, None) => None,
                }
            });
        if let Some(v) = primary {
            *stats.entry(id.to_string()).or_insert(0.0) += v;
        }

        // Every other numeric attribute key: keep it, qualified, so chance
        // percents, reductions, skill levels, and durations aren't lost.
        let min_max_keys_used_as_primary =
            primary.is_some() && !PRIMARY_MAGNITUDE_KEYS.iter().any(|k| attrs.contains_key(*k));
        for (key, val) in attrs {
            if PRIMARY_MAGNITUDE_KEYS.contains(&key.as_str()) {
                continue;
            }
            if min_max_keys_used_as_primary && (key.ends_with("_min") || key.ends_with("_max")) {
                continue; // already folded into the primary magnitude above
            }
            if let Some(v) = parse_num(val) {
                let qualified = format!("{id}:{key}");
                *stats.entry(qualified).or_insert(0.0) += v;
            }
        }
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

/// Max realistic stats a single item can carry that are all relevant to the
/// build: a base item + prefix + suffix can each contribute a couple of
/// stat lines, but real drops rarely hit more than ~4-5 *weighted*
/// priorities at once even with a big priority list — most of an item's
/// affix budget goes to stats you didn't weight. Grading against "if it
/// had literally every weighted stat" (the previous approach) made every
/// real item look weak once more than a few stats were prioritized.
const REALISTIC_HITS_PER_ITEM: usize = 5;

/// The theoretical max score for a strong, realistic item: it hits the
/// `REALISTIC_HITS_PER_ITEM` highest-weighted priorities, each at a solid
/// roll (~30, tuned against typical GD affix magnitudes).
pub fn max_possible_score(weights: &PrioWeights) -> f64 {
    let mut star_weights: Vec<f64> = weights
        .values()
        .filter(|&&w| w > 0)
        .map(|&w| w as f64)
        .collect();
    star_weights.sort_by(|a, b| b.partial_cmp(a).unwrap());
    star_weights
        .into_iter()
        .take(REALISTIC_HITS_PER_ITEM)
        .map(|w| w * 10.0 + 3.0)
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
    /// true if `current_total` already exceeded the cap before the swap
    /// (so losing some of it here isn't actually a real loss — it was
    /// wasted excess already)
    pub was_over_cap_before: bool,
    /// true if `after_total` is at or below 0 (dangerous, taking bonus damage)
    pub dangerous: bool,
    pub effective_cap: f64,
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
            was_over_cap_before: current > effective_cap,
            dangerous: after <= 0.0,
            effective_cap,
        });
    }
    out
}
