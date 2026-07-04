// Fixture: triggers BYPASS_CRATE_ALLOW
// A #![allow(...)] attribute at the crate root suppresses lints globally.

#![allow(dead_code)]
#![allow(clippy::new_without_default)]

pub fn crate_fn() -> i32 {
    0
}
