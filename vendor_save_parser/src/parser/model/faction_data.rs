use serde::{Deserialize, Serialize};

use crate::util::Result;

use super::super::{Readable, Parser};

#[derive(Deserialize, Serialize)]
pub struct FactionData {
    value: f32,
    positive_boost: f32,
    negative_boost: f32,
    modified: u8,
    unlocked: u8,
}

impl Readable for FactionData {
    fn read_from(reader: &mut dyn Parser) -> Result<Self> {
        let modified = reader.read_byte()?;
        let unlocked = reader.read_byte()?;
        let value = reader.read_float()?;
        let positive_boost = reader.read_float()?;
        let negative_boost = reader.read_float()?;

        Ok(FactionData {
            value,
            positive_boost,
            negative_boost,
            modified,
            unlocked,
        })
    }
}
