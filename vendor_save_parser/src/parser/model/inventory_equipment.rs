use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    Item,
};

#[derive(Deserialize, Serialize)]
pub struct InventoryEquipment {
    item: Option<Item>,
    attached: u8,
}

impl Readable for InventoryEquipment {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        let item = Option::read_from(reader)?;
        let attached = reader.read_byte()?;

        Ok(InventoryEquipment { item, attached })
    }
}

impl InventoryEquipment {
    pub fn empty() -> Self {
        InventoryEquipment {
            item: None,
            attached: 0,
        }
    }
}
