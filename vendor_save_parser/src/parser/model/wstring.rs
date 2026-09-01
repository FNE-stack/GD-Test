use std::array;

use serde::{de::Visitor, Deserialize, Serialize};

use crate::util::Result;

use super::super::{Parser, Readable};

pub struct WString {
    v: String,
}

impl WString {
    pub fn empty() -> WString {
        WString { v: String::new() }
    }
}

impl Readable for WString {
    fn read_from(reader: &mut dyn Parser) -> Result<Self>
    where
        Self: Sized,
    {
        let len = reader.read_int()?;
        let mut res: Vec<u16> = Vec::new();
        for _i in 0..len {
            let bytes = array::try_from_fn(|_| reader.read_byte())?;
            res.push(u16::from_le_bytes(bytes));
        }
        Ok(WString {
            v: String::from_utf16(&res).unwrap(),
        })
    }
}

impl Serialize for WString {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.v)
    }
}

impl<'de> Deserialize<'de> for WString {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct StringVisitor;

        impl<'de> Visitor<'de> for StringVisitor {
            type Value = WString;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("string")
            }

            fn visit_string<E>(self, v: String) -> std::result::Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(WString { v })
            }
        }

        deserializer.deserialize_string(StringVisitor)
    }
}
