use crate::parser::{CharacterReader, FormulasReader, Readable, Reader, StashFileReader};
use crate::util::{CustomError, Result};
use serde::Serialize;
use std::io::Read;

fn convert_to_json<U>(obj: U) -> Result<String>
where
    U: Readable + Serialize,
{
    match serde_json::to_string(&obj) {
        Ok(res) => Ok(res),
        Err(e) => Err(CustomError::new(e.to_string())),
    }
}

/// Parses data from `source` based on `entity_type`
///
/// `entity_type` can be one of `character`, `formulas` or `stash`
pub fn map_to_json<T: Read>(entity_type: &str, source: T) -> Result<String> {
    match entity_type {
        "character" => convert_to_json(CharacterReader::read(source)?),
        "formulas" => convert_to_json(FormulasReader::read(source)?),
        "stash" => convert_to_json(StashFileReader::read(source)?),
        _ => Err(CustomError::new("Wrong entity_type".to_owned())),
    }
}
