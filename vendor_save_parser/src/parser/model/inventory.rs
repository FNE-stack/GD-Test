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
        // Not start_block_with_version: this block's version has been
        // observed as high as 11 in the wild (this parser previously only
        // accepted exactly 4), but cross-checked against gd-edit
        // (https://github.com/Odie/gd-edit, an actively-maintained,
        // format-accurate save editor) — its read-block3 reads this exact
        // same field sequence (has-data flag, sack count/focused/selected,
        // sacks, use-alt-weaponset flag, 12 equipment slots, two 2-item
        // weapon sets) with no version-gated fields anywhere in the
        // function. The version is stored but never changes what bytes
        // come next, so asserting a specific value here was only ever
        // going to reject saves from a version bump that changed nothing
        // this struct reads.
        reader.start_block(3)?;
        let _version = reader.read_int()?;

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
