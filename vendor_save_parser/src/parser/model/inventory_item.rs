use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    Item,
};

#[derive(Deserialize, Serialize)]
pub struct InventoryItem {
    item: Item,
    x: u32,
    y: u32,
}

impl Readable for InventoryItem {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        let item = Item::read_from(reader)?;
        let x = reader.read_int()?;
        let y = reader.read_int()?;

        Ok(InventoryItem { item, x, y })
    }
}
