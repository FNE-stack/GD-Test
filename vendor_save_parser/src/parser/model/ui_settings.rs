use std::{array, collections::BTreeMap};

use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Parser, Readable},
    HotSlot,
};

#[derive(Deserialize, Serialize)]
pub struct UISettings {
    hot_slots: BTreeMap<u32, Vec<HotSlot>>,
    unknown4: [String; 5],
    unknown5: [String; 5],
    unknown2: u32,
    camera_distance: f32,
    unknown6: [u8; 5],
    unknown1: u8,
    unknown3: u8,
}

const VERSIONS: [u32; 3] = [5, 6, 7];

impl Readable for UISettings {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        let version = reader.start_block_with_versions(14, &VERSIONS)?;
        let unknown1 = reader.read_byte()?;
        let unknown2 = reader.read_int()?;
        let unknown3 = reader.read_byte()?;
        let mut unknown4 = array::from_fn(|_| String::new());
        let mut unknown5 = array::from_fn(|_| String::new());
        let mut unknown6 = [0; 5];
        for i in 0..5 {
            unknown4[i] = String::read_from(reader)?;
            unknown5[i] = String::read_from(reader)?;
            unknown6[i] = reader.read_byte()?;
        }

        let hot_sets_quantity = if version == 7 {
            reader.read_int()?
        } else { 1 };

        let hot_set_slots = match version {
            5 => 46,
            6 => 47,
            7 => reader.read_int()?,
            _ => 0 // unreachable
        };

        let mut hot_slots = BTreeMap::new();

        for i in 0..hot_sets_quantity {
            let key = if version == 7 {
                reader.read_int()?
            } else {
                i
            };
            let mut hot_set = Vec::new();

            for _j in 0..hot_set_slots {
                hot_set.push(HotSlot::read_from(reader)?);
            }

            hot_slots.insert(key, hot_set);
        }

        let camera_distance = reader.read_float()?;

        reader.end_block()?;

        Ok(UISettings {
            hot_slots,
            unknown4,
            unknown5,
            unknown2,
            camera_distance,
            unknown6,
            unknown1,
            unknown3,
        })
    }
}
