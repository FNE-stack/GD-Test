use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::super::{Readable, Parser};

#[derive(Deserialize, Serialize)]
pub struct TutorialPages {
    pages: Vec<u32>,
}

impl Readable for TutorialPages {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        reader.start_block_with_version(15, 1)?;

        let len = reader.read_int()?;
        let mut pages = Vec::new();
        for _i in 0..len {
            pages.push(reader.read_int()?);
        }

        reader.end_block()?;

        Ok(TutorialPages { pages })
    }
}
