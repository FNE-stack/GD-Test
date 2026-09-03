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

        // skill_bonus/granted_item_skill carry a resolved skill display_name
        // right alongside their magnitude (skill_level for skill_bonus — a
        // plain rank number; granted_item_skill has no clean numeric
        // magnitude at all, since its level_equation is a formula string
        // like "itemLevel/4+1", not a plain number). Special-cased into a
        // stat_id that includes the actual skill name, instead of the
        // generic "{id}:{attribute_key}" qualification below, which would
        // otherwise produce an unreadable "skill_bonus:skill_level" — and
        // silently drop the skill's name entirely, since display_name and
        // skill_reference are non-numeric strings the generic path can't
        // parse into a value, so they'd just vanish rather than qualify
        // anything.
        if id == "skill_bonus" || id == "granted_item_skill" {
            if let Some(name) = attrs.get("display_name").and_then(|v| v.as_str()) {
                // skill_bonus: the real rank being added. granted_item_skill:
                // no such number exists, so 1.0 just marks "this is granted"
                // — the UI labels these two cases differently rather than
                // implying a rank on a granted proc skill.
                let magnitude = attrs.get("skill_level").and_then(parse_num).unwrap_or(1.0);
                let qualified = format!("{id}:{name}");
                *stats.entry(qualified).or_insert(0.0) += magnitude;
                continue;
            }
        }

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

/// Bounds for a single star weight. Matches grim_gleaner's own
/// MIN/MAX_STAT_WEIGHT (domain/profile.py) — kept here as the canonical
/// range so anything accepting weights from outside the UI (e.g. the
/// grim_gleaner profile importer) can clamp against the same bounds the
/// 0-4 star control itself enforces.
pub const MIN_STAR_WEIGHT: u8 = 0;
pub const MAX_STAR_WEIGHT: u8 = 4;

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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn map(pairs: &[(&str, f64)]) -> HashMap<String, f64> {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    fn weights(pairs: &[(&str, u8)]) -> PrioWeights {
        pairs.iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }

    // ---------- extract_stats ----------

    #[test]
    fn primary_magnitude_prefers_percent_key() {
        let resolved = json!({
            "properties": [
                { "property_id": "fire_resistance", "attributes": { "percent": "24.000000" } }
            ]
        });
        let stats = extract_stats(&resolved);
        assert_eq!(stats.get("fire_resistance"), Some(&24.0));
        assert_eq!(stats.len(), 1);
    }

    #[test]
    fn primary_magnitude_averages_min_max_pair_when_no_direct_key() {
        let resolved = json!({
            "properties": [
                {
                    "property_id": "flat_fire_damage",
                    "attributes": { "damage_min": "20.000000", "damage_max": "60.000000" }
                }
            ]
        });
        let stats = extract_stats(&resolved);
        assert_eq!(stats.get("flat_fire_damage"), Some(&40.0));
    }

    #[test]
    fn secondary_attributes_are_kept_qualified_not_dropped() {
        // Neither duration_seconds nor reduction_flat is a primary magnitude
        // key or a _min/_max pair, so both should survive as
        // "property_id:attribute_key" rather than being merged or lost.
        let resolved = json!({
            "properties": [
                {
                    "property_id": "target_resistance_reduction_flat",
                    "attributes": { "duration_seconds": "5.000000", "reduction_flat": "10.000000" }
                }
            ]
        });
        let stats = extract_stats(&resolved);
        assert_eq!(stats.get("target_resistance_reduction_flat"), None);
        assert_eq!(
            stats.get("target_resistance_reduction_flat:duration_seconds"),
            Some(&5.0)
        );
        assert_eq!(
            stats.get("target_resistance_reduction_flat:reduction_flat"),
            Some(&10.0)
        );
    }

    #[test]
    fn primary_and_secondary_attributes_coexist_on_one_property() {
        let resolved = json!({
            "properties": [
                {
                    "property_id": "some_proc",
                    "attributes": { "percent": "15.000000", "chance_percent": "25.000000" }
                }
            ]
        });
        let stats = extract_stats(&resolved);
        assert_eq!(stats.get("some_proc"), Some(&15.0));
        assert_eq!(stats.get("some_proc:chance_percent"), Some(&25.0));
    }

    #[test]
    fn repeated_property_id_across_variants_sums() {
        let resolved = json!({
            "properties": [
                { "property_id": "fire_resistance", "attributes": { "percent": "10.000000" } },
                { "property_id": "fire_resistance", "attributes": { "percent": "6.000000" } }
            ]
        });
        let stats = extract_stats(&resolved);
        assert_eq!(stats.get("fire_resistance"), Some(&16.0));
    }

    #[test]
    fn skill_bonus_resolves_to_the_actual_skill_name_and_rank() {
        let resolved = json!({
            "properties": [
                {
                    "property_id": "skill_bonus",
                    "attributes": {
                        "display_name": "Military Conditioning",
                        "skill_level": "2",
                        "skill_reference": "records/skills/playerclass01/passive1.dbr"
                    }
                }
            ]
        });
        let stats = extract_stats(&resolved);
        assert_eq!(
            stats.get("skill_bonus:Military Conditioning"),
            Some(&2.0)
        );
        // The generic qualified-attribute path shouldn't also fire for this
        // property once the skill special-case has already handled it.
        assert_eq!(stats.get("skill_bonus:skill_level"), None);
    }

    #[test]
    fn granted_item_skill_resolves_to_the_skill_name_with_no_fake_rank() {
        // No skill_level here (granted_item_skill never has one — its
        // level_equation is a formula string, not a plain number), so this
        // should fall back to a flat "granted" marker instead of silently
        // dropping the skill entirely or parsing garbage.
        let resolved = json!({
            "properties": [
                {
                    "property_id": "granted_item_skill",
                    "attributes": {
                        "display_name": "Bloody Pox",
                        "level_equation": "itemLevel/4+1",
                        "skill_reference": "records/skills/itemskills/legendary/item_bloodypox.dbr"
                    }
                }
            ]
        });
        let stats = extract_stats(&resolved);
        assert_eq!(stats.get("granted_item_skill:Bloody Pox"), Some(&1.0));
    }

    #[test]
    fn missing_properties_array_yields_empty_stats() {
        let resolved = json!({ "display_name": "Nothing Here" });
        assert!(extract_stats(&resolved).is_empty());
    }

    // ---------- prio_score ----------

    #[test]
    fn score_combines_star_weight_and_capped_magnitude_bonus() {
        let stats = map(&[("fire_resistance", 24.0)]);
        let w = weights(&[("fire_resistance", 4)]);
        // 4 stars * 10 + min(24, 50) * 0.1
        assert!((prio_score(&stats, &w) - 42.4).abs() < 1e-9);
    }

    #[test]
    fn zero_weight_never_contributes_even_if_present_on_item() {
        let stats = map(&[("junk_stat", 100.0)]);
        let w = weights(&[("junk_stat", 0)]);
        assert_eq!(prio_score(&stats, &w), 0.0);
    }

    #[test]
    fn weighted_stat_missing_from_item_contributes_nothing() {
        let stats = map(&[]);
        let w = weights(&[("fire_resistance", 4)]);
        assert_eq!(prio_score(&stats, &w), 0.0);
    }

    #[test]
    fn weighted_stat_present_but_zero_value_contributes_nothing() {
        let stats = map(&[("cold_resistance", 0.0)]);
        let w = weights(&[("cold_resistance", 4)]);
        assert_eq!(prio_score(&stats, &w), 0.0);
    }

    #[test]
    fn magnitude_bonus_is_capped_at_fifty() {
        let stats = map(&[("fire_damage_percent", 999.0)]);
        let w = weights(&[("fire_damage_percent", 4)]);
        // bonus should clamp to 50 * 0.1 = 5, not 99.9
        assert!((prio_score(&stats, &w) - 45.0).abs() < 1e-9);
    }

    // ---------- letter_grade ----------

    #[test]
    fn grade_boundaries_match_thresholds() {
        assert_eq!(letter_grade(0.0, 0.0), "-");
        assert_eq!(letter_grade(130.0, 100.0), "S++");
        assert_eq!(letter_grade(129.9, 100.0), "S+");
        assert_eq!(letter_grade(110.0, 100.0), "S+");
        assert_eq!(letter_grade(109.9, 100.0), "S");
        assert_eq!(letter_grade(95.0, 100.0), "S");
        assert_eq!(letter_grade(80.0, 100.0), "A");
        assert_eq!(letter_grade(65.0, 100.0), "B");
        assert_eq!(letter_grade(50.0, 100.0), "C");
        assert_eq!(letter_grade(30.0, 100.0), "D");
        assert_eq!(letter_grade(29.9, 100.0), "F");
    }

    #[test]
    fn grade_percent_is_clamped_above_two_hundred() {
        // 300/100*100 = 300%, clamped to 200%, still well above S++.
        assert_eq!(letter_grade(300.0, 100.0), "S++");
    }

    // ---------- max_possible_score ----------

    #[test]
    fn max_possible_score_uses_only_top_five_weights() {
        let w = weights(&[
            ("a", 4),
            ("b", 4),
            ("c", 3),
            ("d", 2),
            ("e", 1),
            ("f", 1), // 6th nonzero weight, excluded by REALISTIC_HITS_PER_ITEM
        ]);
        // (4*10+3)+(4*10+3)+(3*10+3)+(2*10+3)+(1*10+3) = 43+43+33+23+13 = 155
        assert!((max_possible_score(&w) - 155.0).abs() < 1e-9);
    }

    #[test]
    fn max_possible_score_ignores_zero_weights() {
        let w = weights(&[("a", 4), ("ignored", 0)]);
        assert!((max_possible_score(&w) - 43.0).abs() < 1e-9);
    }

    #[test]
    fn max_possible_score_is_zero_for_empty_weights() {
        assert_eq!(max_possible_score(&PrioWeights::new()), 0.0);
    }

    // ---------- resist_impact (item comparison) ----------

    #[test]
    fn compare_two_items_score_and_grade_favor_the_higher_priority_item() {
        // Item A (currently equipped): has nothing the priorities weight.
        let item_a = map(&[("physique", 50.0)]);
        // Item B (candidate): a strong roll of the one weighted priority.
        let item_b = map(&[("fire_resistance", 30.0)]);
        let w = weights(&[("fire_resistance", 4)]);

        // Mirrors the real /api/compare flow: one max_possible computed
        // from the priority set, used to grade both items.
        let max_possible = max_possible_score(&w);
        let score_a = prio_score(&item_a, &w);
        let score_b = prio_score(&item_b, &w);

        assert!(score_b > score_a);
        assert_eq!(letter_grade(score_a, max_possible), "F");
        assert_eq!(letter_grade(score_b, max_possible), "S");
    }

    #[test]
    fn resist_impact_reports_delta_and_effective_cap_with_maximum_bonus() {
        let baseline = map(&[("fire_resistance", 58.0), ("maximum_fire_resistance", 5.0)]);
        let removed = map(&[("fire_resistance", 10.0)]); // item A's contribution
        let added = map(&[("fire_resistance", 30.0)]); // item B's contribution

        let impacts = resist_impact(&baseline, &removed, &added);
        let fire = impacts
            .iter()
            .find(|r| r.stat == "fire_resistance")
            .expect("fire_resistance row present");

        assert_eq!(fire.current_total, 58.0);
        assert_eq!(fire.after_total, 78.0); // 58 - 10 + 30
        assert_eq!(fire.delta, 20.0);
        assert_eq!(fire.effective_cap, 85.0); // 80 default + 5 bonus
        assert!(!fire.over_cap);
        assert!(!fire.was_over_cap_before);
        assert!(!fire.dangerous);
    }

    #[test]
    fn resist_impact_flags_overcap_when_swap_pushes_past_the_cap() {
        let baseline = map(&[("cold_resistance", 70.0)]);
        let removed = map(&[]);
        let added = map(&[("cold_resistance", 20.0)]); // 70 -> 90, cap is 80

        let impacts = resist_impact(&baseline, &removed, &added);
        let cold = impacts.iter().find(|r| r.stat == "cold_resistance").unwrap();
        assert_eq!(cold.after_total, 90.0);
        assert!(cold.over_cap);
        assert!(!cold.was_over_cap_before);
        assert!(!cold.dangerous);
    }

    #[test]
    fn resist_impact_does_not_penalize_losing_already_wasted_overcap_resist() {
        let baseline = map(&[("lightning_resistance", 95.0)]); // already over the 80 cap
        let removed = map(&[("lightning_resistance", 20.0)]); // swap removes some of the excess
        let added = map(&[]);

        let impacts = resist_impact(&baseline, &removed, &added);
        let lightning = impacts
            .iter()
            .find(|r| r.stat == "lightning_resistance")
            .unwrap();
        assert_eq!(lightning.after_total, 75.0);
        assert!(lightning.was_over_cap_before);
        assert!(!lightning.over_cap);
    }

    #[test]
    fn resist_impact_flags_dangerous_when_swap_drops_resist_to_zero_or_below() {
        let baseline = map(&[("aether_resistance", 20.0)]);
        let removed = map(&[("aether_resistance", 25.0)]); // losing more than current has
        let added = map(&[]);

        let impacts = resist_impact(&baseline, &removed, &added);
        let aether = impacts.iter().find(|r| r.stat == "aether_resistance").unwrap();
        assert_eq!(aether.after_total, -5.0);
        assert!(aether.dangerous);
    }

    #[test]
    fn resist_impact_skips_stats_untouched_and_at_zero() {
        let baseline = map(&[("physical_resistance", 0.0)]);
        let removed = map(&[]);
        let added = map(&[]);
        let impacts = resist_impact(&baseline, &removed, &added);
        assert!(impacts.iter().all(|r| r.stat != "physical_resistance"));
    }
}
