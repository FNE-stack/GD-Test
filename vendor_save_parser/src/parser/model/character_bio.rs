use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::super::{Readable, Parser};

#[derive(Deserialize, Serialize)]
pub struct CharacterBio {
    level: u32,
    experience: u32,
    attribute_points_unspent: u32,
    skill_points_unspent: u32,
    devotion_points_unspent: u32,
    total_devotion_unlocked: u32,
    physique: f32,
    cunning: f32,
    spirit: f32,
    health: f32,
    energy: f32,
}

impl Readable for CharacterBio {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        reader.start_block_with_version(2, 8)?;

        let level = reader.read_int()?;
        let experience = reader.read_int()?;
        let attribute_points_unspent = reader.read_int()?;
        let skill_points_unspent = reader.read_int()?;
        let devotion_points_unspent = reader.read_int()?;
        let total_devotion_unlocked = reader.read_int()?;
        let physique = reader.read_float()?;
        let cunning = reader.read_float()?;
        let spirit = reader.read_float()?;
        let health = reader.read_float()?;
        let energy = reader.read_float()?;

        reader.end_block()?;

        Ok(CharacterBio {
            level,
            experience,
            attribute_points_unspent,
            skill_points_unspent,
            devotion_points_unspent,
            total_devotion_unlocked,
            physique,
            cunning,
            spirit,
            health,
            energy,
        })
    }
}
