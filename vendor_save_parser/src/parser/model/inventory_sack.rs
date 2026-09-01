use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    InventoryItem,
};

#[derive(Deserialize, Serialize)]
pub struct InventorySack {
    items: Vec<InventoryItem>,
    temp_bool: u8,
}

impl Readable for InventorySack {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        reader.start_block(0)?;

        let temp_bool = reader.read_byte()?;
        let items = Vec::read_from(reader)?;

        reader.end_block()?;

        Ok(InventorySack { items, temp_bool })
    }
}
