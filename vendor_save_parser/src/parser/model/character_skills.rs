use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Parser, Readable},
    ItemSkill, Skill,
};

#[derive(Deserialize, Serialize)]
pub struct CharacterSkills {
    skills: Vec<Skill>,
    item_skills: Vec<ItemSkill>,
    masteries_allowed: u32,
    skill_reclamation_points_used: u32,
    devotion_reclamation_points_used: u32,
    unknown: Option<u32>,
}

impl Readable for CharacterSkills {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        reader.start_block(8)?;
        let version = reader.read_int()?;

        // This app (only inv.equipment) never reads skill data, and the
        // per-skill layout has grown a field this parser doesn't yet know
        // the size or position of (found empirically: a real save's first
        // Skill entry decodes name/level/enabled/devotion/experience/
        // active/unknown1/unknown2 correctly, then its autocast-skill
        // string reads as garbage — something new was inserted before it
        // that isn't documented in gd-edit's model either). Forward-only
        // reader means we can't safely "try and roll back" a wrong guess
        // here the way we can for a single trailing field (CharacterInfo's
        // loot_mode, StashTab's trailing bytes) — so rather than risk
        // misreading every skill/item-skill in the list, skip straight to
        // this block's declared end and leave the collections empty.
        let remaining = reader.current_block_end().saturating_sub(reader.get_pos()) as usize;
        for _ in 0..remaining {
            reader.read_byte()?;
        }

        reader.end_block()?;

        Ok(CharacterSkills {
            skills: Vec::new(),
            item_skills: Vec::new(),
            masteries_allowed: 0,
            skill_reclamation_points_used: 0,
            devotion_reclamation_points_used: 0,
            unknown: if version >= 6 { Some(0) } else { None },
        })
    }
}
