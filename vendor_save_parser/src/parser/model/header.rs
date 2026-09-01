use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::{
    super::{Readable, Parser},
    WString,
};

#[derive(Deserialize, Serialize)]
pub struct Header {
    name: WString,
    tag: String,
    level: u32,
    sex: u8,
    hardcore: u8,
}

impl Readable for Header {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        let name = WString::read_from(reader)?;
        let sex = reader.read_byte()?;
        let tag = String::read_from(reader)?;
        let level = reader.read_int()?;
        let hardcore = reader.read_byte()?;
        Ok(Header {
            name,
            tag,
            level,
            sex,
            hardcore,
        })
    }
}
