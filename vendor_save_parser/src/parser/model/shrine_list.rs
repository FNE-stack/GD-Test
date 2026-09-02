use std::array;

use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    UID,
};

#[derive(Deserialize, Serialize)]
pub struct ShrineList {
    uids: [Vec<UID>; 6],
}

impl Readable for ShrineList {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        // Not start_block_with_version: cross-checked against gd-edit
        // (https://github.com/Odie/gd-edit) — this block has no
        // version-gated fields at all.
        reader.start_block(17)?;
        let _version = reader.read_int()?;

        let uids = array::try_from_fn(|_| Vec::read_from(reader))?;

        reader.end_block()?;

        Ok(ShrineList { uids })
    }
}
