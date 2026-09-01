use std::array;

use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    UID,
};

#[derive(Deserialize, Serialize)]
pub struct RespawnList {
    uids: [Vec<UID>; 3],
    spawn: [UID; 3],
}

impl Readable for RespawnList {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        reader.start_block_with_version(5, 1)?;

        let uids = array::try_from_fn(|_| Vec::read_from(reader))?;
        let spawn = array::try_from_fn(|_| UID::read_from(reader))?;

        reader.end_block()?;

        Ok(RespawnList { uids, spawn })
    }
}
