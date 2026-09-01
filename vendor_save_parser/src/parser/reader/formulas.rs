use std::cell::RefMut;
use std::{cell::RefCell, io::Read};

use crate::util::Result;

use super::Reader;

use crate::parser::{
    model::FormulaSet,
    parser::{Block, Parser},
    Readable,
};

pub struct FormulasReader<T: Read> {
    source: RefCell<T>,
}

impl<T: Read> Reader for FormulasReader<T> {
    type Item = FormulaSet;
    type Source = T;

    fn read(source: Self::Source) -> Result<Self::Item> {
        let mut reader = FormulasReader {
            source: RefCell::new(source),
        };
        FormulaSet::read_from(&mut reader)
    }
}

impl<T: Read> Parser for FormulasReader<T> {
    fn get_source(&self) -> RefMut<dyn Read> {
        self.source.borrow_mut()
    }
    fn get_key(&self) -> u32 {
        0
    }
    fn get_pos(&self) -> u64 {
        0
    }
    fn inc_pos(&mut self, _v: u64) {}
    fn set_key(&mut self, _key: u32) {}
    fn pop_block(&mut self) -> Result<Block> {
        Ok(Block::new())
    }
    fn push_block(&mut self, _b: Block) {}
    fn get_table_elem(&self, _index: usize) -> u32 {
        0
    }
    fn update_key(&mut self, _ptr: &[u8], _len: u32) {}
    fn set_table_elem(&mut self, _index: usize, _elem: u32) {}
}
