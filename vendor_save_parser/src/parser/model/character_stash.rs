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
        // Not start_block_with_version: cross-checked against gd-edit
        // (https://github.com/Odie/gd-edit) — this block has no
        // version-gated fields at all.
        reader.start_block(4)?;
        let _version = reader.read_int()?;

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
