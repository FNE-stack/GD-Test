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

const VERSIONS: [u32; 2] = [5, 6];

impl Readable for CharacterSkills {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        let version = reader.start_block_with_versions(8, &VERSIONS)?;

        let skills = Vec::read_from(reader)?;
        let masteries_allowed = reader.read_int()?;
        let skill_reclamation_points_used = reader.read_int()?;
        let devotion_reclamation_points_used = reader.read_int()?;
        let item_skills = Vec::read_from(reader)?;
        let unknown = if version == 6 {
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
