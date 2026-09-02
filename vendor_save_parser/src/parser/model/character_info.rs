use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::super::{Parser, Readable};

#[derive(Deserialize, Serialize)]
pub struct CharacterInfo {
    texture: String,
    money: u32,
    // Loot filter settings. Each byte should be either 0 or 1. This used to
    // be a hand-counted fixed-size array (39 bytes, enumerated by category
    // below), but Grim Dawn has grown the in-game loot filter's checkbox
    // categories across patches, and a fixed count silently desyncs the
    // rest of the parse the next time it grows again. Nothing in this
    // crate reads these values, so instead of guessing the current count
    // we just read however many bytes remain in this block — correct
    // regardless of how many toggles exist now or get added later. Known
    // categories as of the last time this was counted:
    // Quality: Common, Magical, Rare, Monster Infrequent, Epic, Legendary, Sets, Always Show Uniques
    // Type: 1h Melee, 2h Melee, 1h Ranged, 2h Ranged, Dagger/Scepter, Caster Off-Hand, Shield, Armor, Accessories, Components
    // Damage: Physical, Pierce, Fire, Cold, Lightning, Acid, Vitality, Aether, Chaos, Bleed, Pet Bonuses
    // Player: My Masteries, Other Masteries, Speed, Cooldown Reduction, Crit Damage, Offensive Ability, Defensive Ability, Resistances, Retaliation
    // Other: Always Show Double Rare
    loot_mode: Vec<u8>,
    current_tribute: u32,
    unknown: u32,
    is_in_main_quest: u8,
    has_been_in_game: u8,
    difficulty: u8,
    greatest_difficulty: u8,
    greatest_survival_difficulty: u8,
    compass_state: u8,
    skill_window_show_help: u8,
    weapon_swap_active: u8,
    weapon_swap_enabled: u8,
}

impl Readable for CharacterInfo {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        // Not start_block_with_version: cross-checked against gd-edit
        // (https://github.com/Odie/gd-edit), whose equivalent block reads
        // an extra int32 only for versions 2-4 (a field this struct never
        // had) and the loot-filter array only for version 5+ — since
        // loot_mode above already reads "whatever's left in the block"
        // instead of a fixed count, an unexpected version 2-4's extra int32
        // just gets silently absorbed into loot_mode's bytes (harmless,
        // since nothing reads its contents) rather than desyncing the read.
        reader.start_block(1)?;
        let _version = reader.read_int()?;

        let is_in_main_quest = reader.read_byte()?;
        let has_been_in_game = reader.read_byte()?;
        let difficulty = reader.read_byte()?;
        let greatest_difficulty = reader.read_byte()?;
        let money = reader.read_int()?;
        let greatest_survival_difficulty = reader.read_byte()?;
        let current_tribute = reader.read_int()?;
        let compass_state = reader.read_byte()?;
        let skill_window_show_help = reader.read_byte()?;
        let weapon_swap_active = reader.read_byte()?;
        let weapon_swap_enabled = reader.read_byte()?;
        let texture = String::read_from(reader)?;
        let unknown = reader.read_int()?;
        let loot_mode_len = reader.current_block_end().saturating_sub(reader.get_pos()) as usize;
        let mut loot_mode = Vec::with_capacity(loot_mode_len);
        for _ in 0..loot_mode_len {
            loot_mode.push(reader.read_byte()?);
        }

        reader.end_block()?;

        Ok(CharacterInfo {
            texture,
            money,
            loot_mode,
            current_tribute,
            unknown,
            is_in_main_quest,
            has_been_in_game,
            difficulty,
            greatest_difficulty,
            greatest_survival_difficulty,
            compass_state,
            skill_window_show_help,
            weapon_swap_active,
            weapon_swap_enabled,
        })
    }
}
