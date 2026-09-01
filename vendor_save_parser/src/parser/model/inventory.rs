use std::array;

use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    InventoryEquipment, InventorySack,
};

#[derive(Deserialize, Serialize)]
pub struct Inventory {
    num_bags: u32,
    sacks: Vec<InventorySack>,
    equipment: [InventoryEquipment; 12],
    weapon1: [InventoryEquipment; 2],
    weapon2: [InventoryEquipment; 2],
    focused: u32,
    selected: u32,
    flag: u8,
    use_alternate: u8,
    alternate1: u8,
    alternate2: u8,
}

impl Readable for Inventory {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        reader.start_block_with_version(3, 4)?;

        let flag = reader.read_byte()?;
        let result = if flag != 0 {
            let num_bags = reader.read_int()?;
            let focused = reader.read_int()?;
            let selected = reader.read_int()?;
            let mut sacks = Vec::new();
            for _i in 0..num_bags {
                sacks.push(InventorySack::read_from(reader)?);
            }
            let use_alternate = reader.read_byte()?;
            let equipment = array::try_from_fn(|_| InventoryEquipment::read_from(reader))?;
            let alternate1 = reader.read_byte()?;
            let weapon1 = array::try_from_fn(|_| InventoryEquipment::read_from(reader))?;
            let alternate2 = reader.read_byte()?;
            let weapon2 = array::try_from_fn(|_| InventoryEquipment::read_from(reader))?;
            Inventory {
                num_bags,
                sacks,
                equipment,
                weapon1,
                weapon2,
                focused,
                selected,
                flag,
                use_alternate,
                alternate1,
                alternate2,
            }
        } else {
            Inventory {
                num_bags: 0,
                sacks: vec![],
                equipment: array::from_fn(|_| InventoryEquipment::empty()),
                weapon1: array::from_fn(|_| InventoryEquipment::empty()),
                weapon2: array::from_fn(|_| InventoryEquipment::empty()),
                focused: 0,
                selected: 0,
                flag,
                use_alternate: 0,
                alternate1: 0,
                alternate2: 0,
            }
        };

        reader.end_block()?;

        Ok(result)
    }
}
