// Fixture: allowed — explicit values, no ambient shared state.
// Write-once memoization over frozen input (LazyLock) is permitted;
// global assignment, locks, and cells are not.

use std::sync::LazyLock;

static ROWS: LazyLock<Vec<&'static str>> = LazyLock::new(|| Vec::new());

pub fn rows() -> &'static [&'static str] {
    &ROWS
}
