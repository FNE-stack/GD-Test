use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::super::{Readable, Parser};

#[derive(Deserialize, Serialize)]
pub struct Skill {
    name: String,
    auto_cast_skill: String,
    auto_cast_controller: String,
    level: u32,
    devotion_level: u32,
    experience: u32,
    active: u32,
    enabled: u8,
    unknown1: u8,
    unknown2: u8,
}

impl Readable for Skill {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        let name = String::read_from(reader)?;
        let level = reader.read_int()?;
        let enabled = reader.read_byte()?;
        let devotion_level = reader.read_int()?;
        let experience = reader.read_int()?;
        let active = reader.read_int()?;
        let unknown1 = reader.read_byte()?;
        let unknown2 = reader.read_byte()?;
        let auto_cast_skill = String::read_from(reader)?;
        let auto_cast_controller = String::read_from(reader)?;

        Ok(Skill {
            name,
            auto_cast_skill,
            auto_cast_controller,
            level,
            devotion_level,
            experience,
            active,
            enabled,
            unknown1,
            unknown2,
        })
    }
}
