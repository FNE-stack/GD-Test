use std::array;

use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

use crate::util::Result;

use super::super::{Parser, Readable};

#[derive(Deserialize, Serialize)]
pub struct CharacterInfo {
    texture: String,
    money: u32,
    // Loot filter settings. Each read byte should be either 0 or 1
    // Quality
    // Common, Magical, Rare, Monster Infrequent, Epic, Legendary, Sets, Always Show Uniques
    // Type
    // 1h Melee, 2h Melee, 1h Ranged, 2h Ranged, Dagger/Scepter, Caster Off-Hand, Shield, Armor, Accessories, Components
    // Damage
    // Physical, Pierce, Fire, Cold, Lightning, Acid, Vitality, Aether, Chaos, Bleed, Pet Bonuses
    // Player
    // My Masteries, Other Masteries, Speed, Cooldown Reduction, Crit Damage, Offensive Ability, Defensive Ability, Resistances, Retaliation
    // Other
    // Always Show Double Rare
    #[serde(with = "BigArray")]
    loot_mode: [u8; 39],
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
        reader.start_block_with_version(1, 5)?;

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
        let loot_mode = array::try_from_fn(|_| reader.read_byte())?;

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
