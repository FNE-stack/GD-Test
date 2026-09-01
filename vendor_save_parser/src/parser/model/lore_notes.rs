use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::super::{Readable, Parser};

#[derive(Deserialize, Serialize)]
pub struct LoreNotes {
    names: Vec<String>,
}

impl Readable for LoreNotes {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        reader.start_block_with_version(12, 1)?;

        let names = Vec::read_from(reader)?;

        reader.end_block()?;

        Ok(LoreNotes { names })
    }
}
