use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    StashItem,
};

#[derive(Deserialize, Serialize)]
pub struct StashTab {
    items: Vec<StashItem>,
    width: u32,
    height: u32,
}

impl Readable for StashTab {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        reader.start_block(0)?;

        let width = reader.read_int()?;
        let height = reader.read_int()?;
        let items = Vec::read_from(reader)?;
        // Not currently used by this app (only inv.equipment is), so
        // rather than guess at whatever trailing field(s) a newer game
        // version added here, just consume however many bytes remain in
        // this block — same approach as CharacterInfo's loot_mode.
        let trailing_len = reader.current_block_end().saturating_sub(reader.get_pos()) as usize;
        for _ in 0..trailing_len {
            reader.read_byte()?;
        }

        reader.end_block()?;

        Ok(StashTab {
            items,
            width,
            height,
        })
    }
}
