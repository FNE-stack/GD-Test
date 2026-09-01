use std::array;

use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::super::{Readable, Parser};

#[derive(Deserialize, Serialize)]
pub struct UID {
    id: [u8; 16],
}

impl Readable for UID {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        let id = array::try_from_fn(|_| reader.read_byte())?;
        Ok(UID { id })
    }
}
