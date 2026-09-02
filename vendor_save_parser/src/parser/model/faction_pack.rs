use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    FactionData,
};

#[derive(Deserialize, Serialize)]
pub struct FactionPack {
    factions: Vec<FactionData>,
    faction: u32,
}

impl Readable for FactionPack {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        // Not start_block_with_version: cross-checked against gd-edit
        // (https://github.com/Odie/gd-edit) — this block has no
        // version-gated fields at all.
        reader.start_block(13)?;
        let _version = reader.read_int()?;

        let faction = reader.read_int()?;
        let factions = Vec::read_from(reader)?;

        reader.end_block()?;

        Ok(FactionPack { factions, faction })
    }
}
