use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::super::{Readable, Parser};

#[derive(Deserialize, Serialize)]
pub struct Item {
    base_name: String,
    prefix_name: String,
    suffix_name: String,
    modifier_name: String,
    transmute_name: String,
    component_name: String,
    relic_bonus: String,
    augment_name: String,
    stack_count: u32,
    seed: u32,
    component_seed: u32,
    unknown: u32,
    augment_seed: u32,
    var1: u32,
}

impl Readable for Item {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        let base_name = String::read_from(reader)?;
        let prefix_name = String::read_from(reader)?;
        let suffix_name = String::read_from(reader)?;
        let modifier_name = String::read_from(reader)?;
        let transmute_name = String::read_from(reader)?;
        let seed = reader.read_int()?;
        let component_name = String::read_from(reader)?;
        let relic_bonus = String::read_from(reader)?;
        let component_seed = reader.read_int()?;
        let augment_name = String::read_from(reader)?;
        let unknown = reader.read_int()?;
        let augment_seed = reader.read_int()?;
        let var1 = reader.read_int()?;
        let stack_count = reader.read_int()?;

        Ok(Item {
            base_name,
            prefix_name,
            suffix_name,
            modifier_name,
            transmute_name,
            component_name,
            relic_bonus,
            augment_name,
            stack_count,
            seed,
            component_seed,
            unknown,
            augment_seed,
            var1,
        })
    }
}
