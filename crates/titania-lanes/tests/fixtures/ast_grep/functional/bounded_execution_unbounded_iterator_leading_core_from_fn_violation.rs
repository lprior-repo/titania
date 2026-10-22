pub fn forever() -> impl Iterator<Item = u32> {
    ::core::iter::from_fn(|| Some(1))
}
