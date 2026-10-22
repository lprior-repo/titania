use std::iter::{self};

pub fn forever_bytes() -> Vec<u8> {
    iter::repeat::<u8>(7).take(2).collect()
}
