use std::array;

use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::super::{Readable, Parser};

#[derive(Deserialize, Serialize)]
pub struct TriggerTokens {
    tokens: [Vec<String>; 3],
}

impl Readable for TriggerTokens {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        reader.start_block_with_version(10, 2)?;

        let tokens = array::try_from_fn(|_| Vec::read_from(reader))?;

        reader.end_block()?;

        Ok(TriggerTokens { tokens })
    }
}
