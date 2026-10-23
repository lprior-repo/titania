pub fn forever_bytes() -> Vec<u8> {
    std::iter::repeat::<u8>(7).take(2).collect()
}
