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
        // Not start_block_with_version: cross-checked against gd-edit
        // (https://github.com/Odie/gd-edit) — this block has no
        // version-gated fields at all.
        reader.start_block(10)?;
        let _version = reader.read_int()?;

        let tokens = array::try_from_fn(|_| Vec::read_from(reader))?;

        reader.end_block()?;

        Ok(TriggerTokens { tokens })
    }
}
