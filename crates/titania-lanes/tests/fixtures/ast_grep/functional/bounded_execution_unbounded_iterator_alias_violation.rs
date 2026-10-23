use std::iter::repeat as rep;

pub fn forever_bytes() -> Vec<u8> {
    rep(7u8).take(2).collect()
}
