use std::iter;

pub fn forever_bytes() -> Vec<u8> {
    iter::repeat(7u8).take(2).collect()
}
