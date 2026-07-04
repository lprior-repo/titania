// Fixture: triggers BYPASS_CFG_ATTR_ALLOW
// A #[cfg_attr(..., allow(...))] conditionally applies an allow attribute.

#[cfg_attr(test, allow(unused_variables))]
pub fn with_cfg_test(_arg: i32) -> i32 {
    42
}

#[cfg_attr(coverage, allow(dead_code))]
pub fn coverage_fn() -> &'static str {
    "hello"
}
