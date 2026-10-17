// Fixture: allowed — synchronous deterministic core calculation.
// Async execution lives in the shell; core computes over explicit inputs.

pub fn total(items: &[i32]) -> i32 {
    items.iter().sum()
}
