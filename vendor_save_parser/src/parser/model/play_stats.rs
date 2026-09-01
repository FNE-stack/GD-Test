use std::array;

use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::super::{Readable, Parser};

#[derive(Deserialize, Serialize)]
pub struct PlayStats {
    greatest_monster_killed_name: [String; 3],
    last_monster_hit: [String; 3],
    last_monster_hit_by: [String; 3],
    greatest_monster_killed_level: [u32; 3],
    greatest_monster_killed_life_and_mana: [u32; 3],
    boss_kills: [u32; 3],
    play_time: u32,
    deaths: u32,
    kills: u32,
    experience_from_kills: u32,
    health_potions_used: u32,
    mana_potions_used: u32,
    max_level: u32,
    hits_received: u32,
    hits_inflicted: u32,
    critical_hits_inflicted: u32,
    critical_hits_received: u32,
    champion_kills: u32,
    hero_kills: u32,
    items_crafted: u32,
    relics_crafted: u32,
    transcendent_relics_crafted: u32,
    mythical_relics_crafted: u32,
    shrines_restored: u32,
    one_shot_chests_opened: u32,
    lore_notes_collected: u32,
    greatest_damage_inflicted: f32,
    last_hit: f32,
    last_hit_by: f32,
    greatest_damage_received: f32,
    survival_wave_tier: u32,
    greatest_survival_score: u32,
    cooldown_remaining: u32,
    cooldown_total: u32,
    v_length: u32,
    v: Vec<UnknownData>,
    shattered_realm_souls: u32,
    shattered_realm_essence: u32,
    difficulty_skip: u8,
    unknown1: u32,
    unknown2: u32,
}

impl Readable for PlayStats {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        reader.start_block_with_version(16, 11)?;

        let play_time = reader.read_int()?;
        let deaths = reader.read_int()?;
        let kills = reader.read_int()?;
        let experience_from_kills = reader.read_int()?;
        let health_potions_used = reader.read_int()?;
        let mana_potions_used = reader.read_int()?;
        let max_level = reader.read_int()?;
        let hits_received = reader.read_int()?;
        let hits_inflicted = reader.read_int()?;
        let critical_hits_inflicted = reader.read_int()?;
        let critical_hits_received = reader.read_int()?;
        let greatest_damage_inflicted = reader.read_float()?;

        let mut greatest_monster_killed_name = array::from_fn(|_| String::new());
        let mut greatest_monster_killed_level = [0; 3];
        let mut greatest_monster_killed_life_and_mana = [0; 3];
        let mut last_monster_hit = array::from_fn(|_| String::new());
        let mut last_monster_hit_by = array::from_fn(|_| String::new());
        for i in 0..3 {
            greatest_monster_killed_name[i] = String::read_from(reader)?;
            greatest_monster_killed_level[i] = reader.read_int()?;
            greatest_monster_killed_life_and_mana[i] = reader.read_int()?;
            last_monster_hit[i] = String::read_from(reader)?;
            last_monster_hit_by[i] = String::read_from(reader)?;
        }

        let champion_kills = reader.read_int()?;
        let last_hit = reader.read_float()?;
        let last_hit_by = reader.read_float()?;
        let greatest_damage_received = reader.read_float()?;
        let hero_kills = reader.read_int()?;
        let items_crafted = reader.read_int()?;
        let relics_crafted = reader.read_int()?;
        let transcendent_relics_crafted = reader.read_int()?;
        let mythical_relics_crafted = reader.read_int()?;
        let shrines_restored = reader.read_int()?;
        let one_shot_chests_opened = reader.read_int()?;
        let lore_notes_collected = reader.read_int()?;

        let boss_kills = array::try_from_fn(|_| reader.read_int())?;

        let survival_wave_tier = reader.read_int()?;
        let greatest_survival_score = reader.read_int()?;
        let cooldown_remaining = reader.read_int()?;
        let cooldown_total = reader.read_int()?;

        let v_length = reader.read_int()?;
        let mut v = Vec::new();
        for _i in 0..v_length {
            let str = String::read_from(reader)?;
            let num = reader.read_int()?;
            v.push(UnknownData { str, num });
        }

        let shattered_realm_souls = reader.read_int()?;
        let shattered_realm_essence = reader.read_int()?;
        let difficulty_skip = reader.read_byte()?;

        let unknown1 = reader.read_int()?;
        let unknown2 = reader.read_int()?;

        reader.end_block()?;

        Ok(PlayStats {
            greatest_monster_killed_name,
            last_monster_hit,
            last_monster_hit_by,
            greatest_monster_killed_level,
            greatest_monster_killed_life_and_mana,
            boss_kills,
            play_time,
            deaths,
            kills,
            experience_from_kills,
            health_potions_used,
            mana_potions_used,
            max_level,
            hits_received,
            hits_inflicted,
            critical_hits_inflicted,
            critical_hits_received,
            champion_kills,
            hero_kills,
            items_crafted,
            relics_crafted,
            transcendent_relics_crafted,
            mythical_relics_crafted,
            shrines_restored,
            one_shot_chests_opened,
            lore_notes_collected,
            greatest_damage_inflicted,
            last_hit,
            last_hit_by,
            greatest_damage_received,
            survival_wave_tier,
            greatest_survival_score,
            cooldown_remaining,
            cooldown_total,
            v_length,
            v,
            shattered_realm_souls,
            shattered_realm_essence,
            difficulty_skip,
            unknown1,
            unknown2,
        })
    }
}

#[derive(Deserialize, Serialize)]
struct UnknownData {
    str: String,
    num: u32,
}
