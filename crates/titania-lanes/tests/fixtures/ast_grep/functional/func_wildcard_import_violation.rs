// Fixture: triggers FUNC_WILDCARD_IMPORT
// Wildcard imports in production source — should use explicit imports.

use std::collections::*;
use std::io::Write;
use crate::types::*;

pub fn make_map() -> HashMap<String, i32> {
    let mut m = HashMap::new();
    m.insert("key".to_string(), 42);
    m
}
