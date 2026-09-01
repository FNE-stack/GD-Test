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
        reader.start_block_with_version(13, 5)?;

        let faction = reader.read_int()?;
        let factions = Vec::read_from(reader)?;

        reader.end_block()?;

        Ok(FactionPack { factions, faction })
    }
}
