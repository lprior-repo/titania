
pub fn repeat_forever() -> impl Iterator<Item = u32> {
    std::iter::repeat(1u32)
}

pub fn cycle_forever(items: Vec<u32>) -> impl Iterator<Item = u32> {
    items.into_iter().cycle()
}
