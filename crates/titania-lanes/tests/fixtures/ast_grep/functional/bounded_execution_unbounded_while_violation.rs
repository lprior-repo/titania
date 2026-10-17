// Fixture: triggers BOUNDED_EXECUTION_UNBOUNDED_LOOP via a condition
// `while` loop (no `while let`, no `loop` block). The first match must land
// on the `while` keyword.

pub fn countdown(start: u64) -> u64 {
    let mut remaining = start;
    while remaining > 0 {
        remaining -= 1;
    }
    remaining
}
