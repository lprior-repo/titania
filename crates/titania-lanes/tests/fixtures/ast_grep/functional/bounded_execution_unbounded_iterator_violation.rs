// Fixture: triggers BOUNDED_EXECUTION_UNBOUNDED_ITERATOR
// Unbounded stream written without loop syntax: infinite iterator sources
// (`repeat`, `.cycle()`) with no `take` bound and no checked-in termination
// claim. Combinator syntax never establishes boundedness.

pub fn repeat_forever() -> impl Iterator<Item = u32> {
    std::iter::repeat(1u32)
}

pub fn cycle_forever(items: Vec<u32>) -> impl Iterator<Item = u32> {
    items.into_iter().cycle()
}
