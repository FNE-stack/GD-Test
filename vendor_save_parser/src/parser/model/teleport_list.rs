use std::array;

use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    UID,
};

#[derive(Deserialize, Serialize)]
pub struct TeleportList {
    uids: [Vec<UID>; 3],
}

impl Readable for TeleportList {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        reader.start_block_with_version(6, 1)?;

        let uids = array::try_from_fn(|_| Vec::read_from(reader))?;

        reader.end_block()?;

        Ok(TeleportList { uids })
    }
}
