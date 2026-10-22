pub fn forever() -> impl Iterator<Item = u32> {
    ::std::iter::successors(Some(1), |n| Some(*n + 1))
}
