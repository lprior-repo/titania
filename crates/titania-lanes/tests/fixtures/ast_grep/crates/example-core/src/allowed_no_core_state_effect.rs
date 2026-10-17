
use std::sync::LazyLock;

static ROWS: LazyLock<Vec<&'static str>> = LazyLock::new(|| Vec::new());

pub fn rows() -> &'static [&'static str] {
    &ROWS
}
