use crate::util::Result;

use super::Parser;

pub trait Readable {
    fn read_from(reader: &mut dyn Parser) -> Result<Self>
    where
        Self: Sized;
}

impl<T> Readable for Vec<T>
where
    T: Readable,
{
    fn read_from(reader: &mut dyn Parser) -> Result<Self>
    where
        Self: Sized,
    {
        let len = reader.read_int()?;
        let mut res = Vec::new();
        for _i in 0..len {
            res.push(T::read_from(reader)?);
        }
        Ok(res)
    }
}

impl<T> Readable for Option<T>
where
    T: Readable,
{
    fn read_from(reader: &mut dyn Parser) -> Result<Self>
    where
        Self: Sized,
    {
        T::read_from(reader).map(|r| Some(r))
    }
}

impl Readable for String {
    fn read_from(reader: &mut dyn Parser) -> Result<Self>
    where
        Self: Sized,
    {
        let len = reader.read_int()?;
        let mut res: Vec<u8> = Vec::new();
        for _i in 0..len {
            res.push(reader.read_byte()?);
        }
        Ok(String::from_utf8(res)?)
    }
}
