use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::super::{Readable, Parser};

#[derive(Deserialize, Serialize)]
pub struct ItemSkill {
    name: String,
    auto_cast_skill: String,
    auto_cast_controller: String,
    item_name: String,
    item_slot: u32,
}

impl Readable for ItemSkill {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        let name = String::read_from(reader)?;
        let auto_cast_skill = String::read_from(reader)?;
        let auto_cast_controller = String::read_from(reader)?;
        let item_slot = reader.read_int()?;
        let item_name = String::read_from(reader)?;
        Ok(ItemSkill {
            name,
            auto_cast_skill,
            auto_cast_controller,
            item_name,
            item_slot,
        })
    }
}
