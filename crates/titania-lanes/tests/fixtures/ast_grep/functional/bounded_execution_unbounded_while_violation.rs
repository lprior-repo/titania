
pub fn countdown(start: u64) -> u64 {
    let mut remaining = start;
    while remaining > 0 {
        remaining -= 1;
    }
    remaining
}
