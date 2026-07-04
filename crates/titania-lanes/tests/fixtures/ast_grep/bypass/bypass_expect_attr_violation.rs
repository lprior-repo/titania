// Fixture: triggers BYPASS_EXPECT_ATTR
// An #[expect(...)] attribute (RFC-641) suppresses a future lint.

use std::fmt;

#[expect(dead_code)]
struct UnusedStruct {
    field: i32,
}

#[expect(clippy::needless_pass_by_value)]
pub fn takes_owned(_s: String) {}
