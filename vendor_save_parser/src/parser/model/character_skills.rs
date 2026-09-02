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
        // Not start_block_with_versions: cross-checked against gd-edit
        // (https://github.com/Odie/gd-edit), whose equivalent block gates
        // its one extra trailing field on "version >= 6", not "version ==
        // 6" — so a version past 6 still has this field, but the old
        // exact-list check here ([5, 6]) rejected it outright before ever
        // getting to read it.
        reader.start_block(8)?;
        let version = reader.read_int()?;

        let skills = Vec::read_from(reader)?;
        let masteries_allowed = reader.read_int()?;
        let skill_reclamation_points_used = reader.read_int()?;
        let devotion_reclamation_points_used = reader.read_int()?;
        let item_skills = Vec::read_from(reader)?;
        let unknown = if version >= 6 {
            Some(reader.read_int()?)
        } else {
            None
        };

        reader.end_block()?;

        Ok(CharacterSkills {
            skills,
            item_skills,
            masteries_allowed,
            skill_reclamation_points_used,
            devotion_reclamation_points_used,
            unknown,
        })
    }
}
