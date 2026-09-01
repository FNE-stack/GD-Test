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

        reader.end_block()?;

        Ok(StashTab {
            items,
            width,
            height,
        })
    }
}
