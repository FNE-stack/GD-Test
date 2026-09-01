use std::{cell::RefMut, io::Read};

use crate::read_exact;
use crate::util::{ensure_contains, ensure_eq, Result};

pub trait Parser {
    fn get_source(&self) -> RefMut<dyn Read>;
    fn get_key(&self) -> u32;
    fn get_table_elem(&self, index: usize) -> u32;
    fn set_key(&mut self, key: u32);
    fn set_table_elem(&mut self, index: usize, elem: u32);
    fn push_block(&mut self, b: Block);
    fn pop_block(&mut self) -> Result<Block>;

    fn get_pos(&self) -> u64;
    fn inc_pos(&mut self, v: u64);

    fn read_key(&mut self) -> Result<()> {
        let (k, _) = read_exact!(self.get_source(), self, u32);
        let mut k = k ^ 0x55555555u32;
        self.set_key(k);
        for i in 0..256 {
            k = k.rotate_right(1);
            k = k.wrapping_mul(39916801u32);
            self.set_table_elem(i, k);
        }
        Ok(())
    }
    fn next_int(&mut self) -> Result<u32> {
        let (num, _) = read_exact!(self.get_source(), self, u32);
        Ok(num ^ self.get_key())
    }
    fn update_key(&mut self, ptr: &[u8], len: u32) {
        for i in 0..len {
            self.set_key(self.get_key() ^ self.get_table_elem(ptr[i as usize] as usize));
        }
    }
    fn read_int(&mut self) -> Result<u32> {
        let (num, buff) = read_exact!(self.get_source(), self, u32);
        let result = num ^ self.get_key();
        self.update_key(&buff, 4);
        Ok(result)
    }
    fn _read_short(&mut self) -> Result<u16> {
        let (num, buff) = read_exact!(self.get_source(), self, u16);
        let result = num ^ (self.get_key() as u16);
        self.update_key(&buff, 2);
        Ok(result)
    }
    fn read_byte(&mut self) -> Result<u8> {
        let (num, buff) = read_exact!(self.get_source(), self, u8);
        let result = num ^ (self.get_key() as u8);
        self.update_key(&buff, 1);
        Ok(result)
    }
    fn read_float(&mut self) -> Result<f32> {
        let num = self.read_int()?;
        Ok(f32::from_le_bytes(num.to_le_bytes()))
    }
    fn read_block_start(&mut self, b: &mut Block) -> Result<u32> {
        let result = self.read_int()?;
        b.len = self.next_int()?;
        b.end = self.get_pos() + u64::from(b.len);
        Ok(result)
    }
    fn start_block(&mut self, t: u32) -> Result<()> {
        let mut b = Block::new();
        ensure_eq(self.read_block_start(&mut b)?, t, "block start")?;

        self.push_block(b);
        Ok(())
    }
    fn start_block_with_version(&mut self, t: u32, v: u32) -> Result<()> {
        let mut b = Block::new();
        ensure_eq(
            self.read_block_start(&mut b)?,
            t,
            "block start with version",
        )?;
        ensure_eq(self.read_int()?, v, "version")?;
        self.push_block(b);
        Ok(())
    }
    fn start_block_with_versions(&mut self, t: u32, v: &[u32]) -> Result<u32> {
        let mut b = Block::new();
        ensure_eq(
            self.read_block_start(&mut b)?,
            t,
            "block start with version",
        )?;
        let version = ensure_contains(self.read_int()?, v, "version")?;
        self.push_block(b);
        Ok(version)
    }
    fn end_block(&mut self) -> Result<()> {
        let b = self.pop_block()?;
        ensure_eq(self.get_pos(), b.end, "block end position")?;
        ensure_eq(self.next_int()?, 0, "block end")?;
        Ok(())
    }
}

pub struct Block {
    pub len: u32,
    pub end: u64,
}

impl Block {
    pub fn new() -> Block {
        Block { len: 0, end: 0 }
    }
}
