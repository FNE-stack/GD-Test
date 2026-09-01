use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    StashTab,
};

#[derive(Deserialize, Serialize)]
pub struct CharacterStash {
    tabs: Vec<StashTab>,
    stash_tabs_purchased: u32,
}

impl Readable for CharacterStash {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        reader.start_block_with_version(4, 6)?;

        let stash_tabs_purchased = reader.read_int()?;
        let mut tabs = Vec::new();
        for _i in 0..stash_tabs_purchased {
            tabs.push(StashTab::read_from(reader)?);
        }

        reader.end_block()?;

        Ok(CharacterStash {
            tabs,
            stash_tabs_purchased,
        })
    }
}
