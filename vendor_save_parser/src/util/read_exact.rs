#[macro_export]
macro_rules! read_exact {
    ($f:expr, $r:expr, $t:ident) => {
        {
            let mut buff = [0u8; ($t::BITS / u8::BITS) as usize];
            $f.read_exact(&mut buff)?;
            $r.inc_pos(($t::BITS / u8::BITS) as u64);
            ($t::from_le_bytes(buff), buff)
        }
    };
}
