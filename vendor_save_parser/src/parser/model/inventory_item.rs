use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    Item, UID,
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
        // A 16-byte per-item UID now follows X/Y here — confirmed empirically
        // against a real save (this parser's prior field list was missing
        // it entirely, causing every item after the first in a sack to be
        // read 16 bytes out of alignment). Not exposed on InventoryItem
        // since nothing downstream needs it; consumed just to stay aligned.
        let _uid = UID::read_from(reader)?;

        Ok(InventoryItem { item, x, y })
    }
}
