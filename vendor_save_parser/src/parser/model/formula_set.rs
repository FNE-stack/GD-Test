use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::parser::{Parser, Readable};
use crate::util::{ensure_eq, Result};

#[derive(Deserialize, Serialize)]
pub struct FormulaSet {
    formulas: HashSet<String>,
}

impl Readable for FormulaSet {
    fn read_from(reader: &mut dyn Parser) -> Result<Self>
    where
        Self: Sized,
    {
        let mut formulas = HashSet::<String>::new();

        ensure_eq(
            String::read_from(reader)?,
            "begin_block".to_owned(),
            "block start",
        )?;
        reader.read_int()?;
        ensure_eq(
            String::read_from(reader)?,
            "formulasVersion".to_owned(),
            "formulasVersion string",
        )?;
        let version = reader.read_int()?;
        ensure_eq(
            String::read_from(reader)?,
            "numEntries".to_owned(),
            "numEntries string",
        )?;
        let num_entries = reader.read_int()?;
        if version >= 3 {
            ensure_eq(
                String::read_from(reader)?,
                "expansionStatus".to_owned(),
                "expansionStatus string",
            )?;
            reader.read_byte()?;
        }
        for _ in 0..num_entries {
            ensure_eq(
                String::read_from(reader)?,
                "itemName".to_owned(),
                "itemName string",
            )?;
            formulas.insert(String::read_from(reader)?);
            ensure_eq(
                String::read_from(reader)?,
                "formulaRead".to_owned(),
                "formulaRead string",
            )?;
            reader.read_int()?;
        }
        ensure_eq(
            String::read_from(reader)?,
            "end_block".to_owned(),
            "block end",
        )?;

        formulas.insert("records/items/crafting/blueprints/relic/craft_relic_b001.dbr".to_string());
        formulas.insert("records/items/crafting/blueprints/relic/craft_relic_b002.dbr".to_string());
        formulas.insert("records/items/crafting/blueprints/relic/craft_relic_b003.dbr".to_string());

        Ok(FormulaSet { formulas })
    }
}
