use std::io::Read;

use crate::util::Result;

use super::super::Readable;

pub trait Reader {
    type Source: Read;
    type Item: Readable;

    fn read(source: Self::Source) -> Result<Self::Item>;
}
