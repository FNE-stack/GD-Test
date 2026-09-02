use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    Item, UID,
};

#[derive(Deserialize, Serialize)]
pub struct StashItem {
    item: Item,
    x: f32,
    y: f32,
}

impl Readable for StashItem {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        let item = Item::read_from(reader)?;
        let x = reader.read_float()?;
        let y = reader.read_float()?;
        // Same trailing per-item UID as InventoryItem/InventoryEquipment —
        // confirmed empirically against a real save's stash contents.
        let _uid = UID::read_from(reader)?;

        Ok(StashItem { item, x, y })
    }
}
