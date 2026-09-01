use serde::{Deserialize, Serialize};

use crate::util::{ensure_eq, Result};

use super::{
    super::{Readable, Parser},
    StashTab,
};

#[derive(Deserialize, Serialize)]
pub struct StashFile {
    tabs: Vec<StashTab>,
    r#mod: String,
}

impl Readable for StashFile {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        ensure_eq(reader.read_int()?, 2, "start bytes")?;

        reader.start_block_with_version(18, 5)?;

        ensure_eq(reader.next_int()?, 0, "bytes 1")?;
        let r#mod = String::read_from(reader)?;
        ensure_eq(reader.read_byte()?, 3, "bytes 2")?;
        let tabs = Vec::read_from(reader)?;

        reader.end_block()?;

        Ok(StashFile { tabs, r#mod })
    }
}
