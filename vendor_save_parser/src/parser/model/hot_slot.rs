use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    WString,
};

// 'type' mapping:
//   0: regular skill
//   2: health potion
//   3: energy potion
//   4294967295: empty
// 'slot' mapping:
//   0-9: primary action bar
//    10: primary weapon set left click
//    11: secondary weapon set left click
//    12: primary weapon set right click
//    13  secondary weapon set right click
// 14-23: secondary action bar
//    24: drink health potion
//    25: drink energy potion
//    26: stationary attack?
// 27-35: ?
#[derive(Deserialize, Serialize)]
pub struct HotSlot {
    skill: String,
    item: String,
    bitmap_up: String,
    bitmap_down: String,
    label: WString,
    slot_type: u32,
    equip_location: u32,
    is_item_skill: u8,
}

impl Readable for HotSlot {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        let slot_type = reader.read_int()?;

        let result = if slot_type == 0 {
            let skill = String::read_from(reader)?;
            let is_item_skill = reader.read_byte()?;
            let item = String::read_from(reader)?;
            let equip_location = reader.read_int()?;

            HotSlot {
                skill,
                item,
                bitmap_up: String::new(),
                bitmap_down: String::new(),
                label: WString::empty(),
                slot_type,
                equip_location,
                is_item_skill,
            }
        } else if slot_type == 4 {
            let item = String::read_from(reader)?;
            let bitmap_up = String::read_from(reader)?;
            let bitmap_down = String::read_from(reader)?;
            let label = WString::read_from(reader)?;

            HotSlot {
                skill: String::new(),
                item,
                bitmap_up,
                bitmap_down,
                label,
                slot_type,
                equip_location: 0,
                is_item_skill: 0,
            }
        } else {
            HotSlot {
                skill: String::new(),
                item: String::new(),
                bitmap_up: String::new(),
                bitmap_down: String::new(),
                label: WString::empty(),
                slot_type,
                equip_location: 0,
                is_item_skill: 0,
            }
        };

        Ok(result)
    }
}
