use std::iter::repeat;

pub fn forever_bytes() -> Vec<u8> {
    repeat::<u8>(7).take(2).collect()
}
