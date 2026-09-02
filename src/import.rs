//! Imports a grim_gleaner (https://github.com/kultcher/grim_gleaner) build
//! profile JSON and converts it into GD Gear Compare's own weights format.
//!
//! grim_gleaner profiles and GD Gear Compare's `weights` map use the same
//! stat_id vocabulary (both are built from the same vendored catalog under
//! data/catalog), so the bulk of a profile ports over unchanged. Two
//! grim_gleaner-only concepts don't have an equivalent here yet and are
//! reported back to the caller rather than silently dropped (see the
//! "Not built yet" section of README.md):
//!   - skill_weights (per-skill priority, used for skill-modifier items)
//!   - masteries (used by grim_gleaner to score mastery_bonus stats)
//! resistance_cap_weights, when resistance_cap_enabled is true, DOES have
//! an equivalent: it overrides the plain weight for that stat, since GD
//! Gear Compare only has one weight per stat (no separate "cap mode").

use crate::stats::{PrioWeights, MAX_STAR_WEIGHT, MIN_STAR_WEIGHT};
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Deserialize, Default)]
struct GrimGleanerProfile {
    #[serde(default)]
    name: String,
    #[serde(default)]
    weights: HashMap<String, Value>,
    #[serde(default)]
    resistance_cap_enabled: bool,
    #[serde(default)]
    resistance_cap_weights: HashMap<String, Value>,
    #[serde(default)]
    skill_weights: HashMap<String, Value>,
    #[serde(default)]
    masteries: Vec<Value>,
}

#[derive(Debug, Clone, serde::Serialize, Default)]
pub struct ImportSummary {
    pub profile_name: String,
    pub imported_stat_count: usize,
    pub resistance_overrides_applied: usize,
    pub skipped_skill_weight_count: usize,
    pub skipped_mastery_count: usize,
    /// Weight entries whose value wasn't a finite JSON number and were
    /// dropped entirely (out-of-range but numeric values are clamped
    /// instead — see `parse_weight`).
    pub invalid_weight_count: usize,
}

#[derive(Debug)]
pub struct ImportResult {
    pub weights: PrioWeights,
    pub summary: ImportSummary,
}

/// Parses raw grim_gleaner profile JSON text and converts it to GD Gear
/// Compare's weights map (the only piece of a profile this app currently
/// consumes).
pub fn import_grim_gleaner_profile(body: &str) -> Result<ImportResult, String> {
    let profile: GrimGleanerProfile = serde_json::from_str(body)
        .map_err(|e| format!("not a valid grim_gleaner profile: {e}"))?;

    let mut invalid_weight_count = 0usize;
    let mut weights: PrioWeights = HashMap::new();
    for (stat_id, raw) in &profile.weights {
        match parse_weight(raw) {
            Some(w) => {
                weights.insert(stat_id.clone(), w);
            }
            None => invalid_weight_count += 1,
        }
    }

    let mut resistance_overrides_applied = 0usize;
    if profile.resistance_cap_enabled {
        for (stat_id, raw) in &profile.resistance_cap_weights {
            match parse_weight(raw) {
                Some(w) => {
                    weights.insert(stat_id.clone(), w);
                    resistance_overrides_applied += 1;
                }
                None => invalid_weight_count += 1,
            }
        }
    }

    let skipped_mastery_count = profile
        .masteries
        .iter()
        .filter(|m| m.as_str().is_some_and(|s| !s.trim().is_empty()))
        .count();

    let profile_name = if profile.name.trim().is_empty() {
        "(unnamed profile)".to_string()
    } else {
        profile.name.clone()
    };

    Ok(ImportResult {
        summary: ImportSummary {
            profile_name,
            imported_stat_count: weights.len(),
            resistance_overrides_applied,
            skipped_skill_weight_count: profile.skill_weights.len(),
            skipped_mastery_count,
            invalid_weight_count,
        },
        weights,
    })
}

/// grim_gleaner weights are integers 0-4 (0 is normally omitted from the
/// file entirely, since grim_gleaner's own BuildProfile.set_weight pops
/// zero entries — see domain/profile.py). Parsed defensively anyway: any
/// finite JSON number is rounded and clamped into GD Gear Compare's own
/// 0-4 range rather than trusting an external file's bounds; anything
/// that isn't a finite number (a string, null, NaN, ±infinity) is
/// rejected outright rather than silently coerced to 0.
fn parse_weight(value: &Value) -> Option<u8> {
    let n = value.as_f64()?;
    if !n.is_finite() {
        return None;
    }
    Some(n.round().clamp(MIN_STAR_WEIGHT as f64, MAX_STAR_WEIGHT as f64) as u8)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn imports_plain_weights_unchanged() {
        let body = json!({
            "name": "Fire / Burn Vire's Might Shieldbreaker",
            "weights": { "fire_resistance": 2, "fire_damage_percent": 4 },
            "resistance_cap_enabled": false,
            "resistance_cap_weights": {},
            "skill_weights": {},
            "masteries": ["", ""]
        })
        .to_string();

        let result = import_grim_gleaner_profile(&body).expect("valid profile");
        assert_eq!(result.weights.get("fire_resistance"), Some(&2));
        assert_eq!(result.weights.get("fire_damage_percent"), Some(&4));
        assert_eq!(result.summary.imported_stat_count, 2);
        assert_eq!(result.summary.resistance_overrides_applied, 0);
        assert_eq!(result.summary.skipped_skill_weight_count, 0);
        assert_eq!(result.summary.skipped_mastery_count, 0);
        assert_eq!(result.summary.invalid_weight_count, 0);
        assert_eq!(
            result.summary.profile_name,
            "Fire / Burn Vire's Might Shieldbreaker"
        );
    }

    #[test]
    fn resistance_cap_weights_are_ignored_when_cap_mode_disabled() {
        let body = json!({
            "weights": { "fire_resistance": 2 },
            "resistance_cap_enabled": false,
            "resistance_cap_weights": { "fire_resistance": 4 }
        })
        .to_string();

        let result = import_grim_gleaner_profile(&body).unwrap();
        assert_eq!(result.weights.get("fire_resistance"), Some(&2));
        assert_eq!(result.summary.resistance_overrides_applied, 0);
    }

    #[test]
    fn resistance_cap_weights_override_plain_weight_when_cap_mode_enabled() {
        let body = json!({
            "weights": { "fire_resistance": 2 },
            "resistance_cap_enabled": true,
            "resistance_cap_weights": { "fire_resistance": 4 }
        })
        .to_string();

        let result = import_grim_gleaner_profile(&body).unwrap();
        assert_eq!(result.weights.get("fire_resistance"), Some(&4));
        assert_eq!(result.summary.resistance_overrides_applied, 1);
    }

    #[test]
    fn out_of_range_numeric_weight_is_clamped_not_dropped() {
        let body = json!({ "weights": { "fire_resistance": 9, "cold_resistance": -3 } }).to_string();
        let result = import_grim_gleaner_profile(&body).unwrap();
        assert_eq!(result.weights.get("fire_resistance"), Some(&4));
        assert_eq!(result.weights.get("cold_resistance"), Some(&0));
        assert_eq!(result.summary.invalid_weight_count, 0);
    }

    #[test]
    fn non_numeric_weight_is_dropped_and_counted_invalid() {
        let body = json!({ "weights": { "fire_resistance": "a lot", "cold_resistance": 3 } })
            .to_string();
        let result = import_grim_gleaner_profile(&body).unwrap();
        assert_eq!(result.weights.get("fire_resistance"), None);
        assert_eq!(result.weights.get("cold_resistance"), Some(&3));
        assert_eq!(result.summary.invalid_weight_count, 1);
        assert_eq!(result.summary.imported_stat_count, 1);
    }

    #[test]
    fn skill_weights_and_masteries_are_reported_as_skipped_not_imported() {
        let body = json!({
            "weights": {},
            "skill_weights": {
                "records/skills/playerclass02/passive1.dbr": 3,
                "records/skills/playerclass09/viremight1.dbr": 4
            },
            "masteries": ["playerclass02", "playerclass09"]
        })
        .to_string();

        let result = import_grim_gleaner_profile(&body).unwrap();
        assert!(result.weights.is_empty());
        assert_eq!(result.summary.skipped_skill_weight_count, 2);
        assert_eq!(result.summary.skipped_mastery_count, 2);
    }

    #[test]
    fn empty_mastery_slots_are_not_counted_as_skipped() {
        let body = json!({ "weights": {}, "masteries": ["playerclass02", ""] }).to_string();
        let result = import_grim_gleaner_profile(&body).unwrap();
        assert_eq!(result.summary.skipped_mastery_count, 1);
    }

    #[test]
    fn missing_optional_fields_default_sensibly() {
        let body = json!({ "weights": { "fire_resistance": 2 } }).to_string();
        let result = import_grim_gleaner_profile(&body).unwrap();
        assert_eq!(result.weights.get("fire_resistance"), Some(&2));
        assert_eq!(result.summary.profile_name, "(unnamed profile)");
    }

    #[test]
    fn malformed_json_is_rejected() {
        let err = import_grim_gleaner_profile("not json at all").unwrap_err();
        assert!(err.contains("not a valid grim_gleaner profile"));
    }

    #[test]
    fn top_level_array_is_rejected() {
        let err = import_grim_gleaner_profile("[1, 2, 3]").unwrap_err();
        assert!(err.contains("not a valid grim_gleaner profile"));
    }
}
