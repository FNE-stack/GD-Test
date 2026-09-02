use std::array;

use serde::{Deserialize, Serialize};

use super::{
    super::{Readable, Parser},
    UID,
};
use crate::util::Result;

#[derive(Deserialize, Serialize)]
pub struct MarkerList {
    uids: [Vec<UID>; 3],
}

impl Readable for MarkerList {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        // Not start_block_with_version: cross-checked against gd-edit
        // (https://github.com/Odie/gd-edit) — this block has no
        // version-gated fields at all.
        reader.start_block(7)?;
        let _version = reader.read_int()?;

        let uids = array::try_from_fn(|_| Vec::read_from(reader))?;

        reader.end_block()?;

        Ok(MarkerList { uids })
    }
}
