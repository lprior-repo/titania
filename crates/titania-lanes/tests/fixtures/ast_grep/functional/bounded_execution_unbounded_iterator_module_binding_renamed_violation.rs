use core::iter as ci;

pub fn forever_bytes() -> Vec<u8> {
    ci::repeat_with(|| 7u8).take(2).collect()
}
