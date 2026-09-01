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
        reader.start_block_with_version(7, 1)?;

        let uids = array::try_from_fn(|_| Vec::read_from(reader))?;

        reader.end_block()?;

        Ok(MarkerList { uids })
    }
}
