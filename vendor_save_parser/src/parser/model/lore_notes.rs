use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::super::{Readable, Parser};

#[derive(Deserialize, Serialize)]
pub struct LoreNotes {
    names: Vec<String>,
}

impl Readable for LoreNotes {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        // Not start_block_with_version: cross-checked against gd-edit
        // (https://github.com/Odie/gd-edit) — this block has no
        // version-gated fields at all.
        reader.start_block(12)?;
        let _version = reader.read_int()?;

        let names = Vec::read_from(reader)?;

        reader.end_block()?;

        Ok(LoreNotes { names })
    }
}
