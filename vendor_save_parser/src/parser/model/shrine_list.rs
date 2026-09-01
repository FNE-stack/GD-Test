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
        reader.start_block_with_version(17, 2)?;

        let uids = array::try_from_fn(|_| Vec::read_from(reader))?;

        reader.end_block()?;

        Ok(ShrineList { uids })
    }
}
