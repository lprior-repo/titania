pub fn forever() -> impl Iterator<Item = u32> {
    core::iter::repeat_with(|| 1)
}
