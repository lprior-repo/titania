// H3: for-keyword column accuracy — `for` at 1-based line 4, 0-based col 4.
pub fn sum(values: &[u32]) -> u32 {
    let mut total = 0;
    for v in values {
        total += v;
    }
    total
}
