pub fn forever() -> impl Iterator<Item = u32> {
    std::iter::repeat(0)
}
