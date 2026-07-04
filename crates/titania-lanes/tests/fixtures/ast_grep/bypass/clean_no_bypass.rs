// Fixture: clean — no bypass attributes or inline suppressions.
// This file should NOT trigger any BYPASS_* rule.

use std::collections::HashMap;

pub fn clean_fn(map: &HashMap<String, i32>) -> Option<i32> {
    map.get("key").copied()
}

pub fn safe_unwrap(opt: Option<&str>) -> &str {
    opt.unwrap_or("default")
}
