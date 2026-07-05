// Fixture: allowed — no infrastructure imports in core/domain.
// Core code using only standard library and domain types.

use std::collections::HashMap;

pub struct UserRepository {
    data: HashMap<String, String>,
}

impl UserRepository {
    pub fn new() -> Self {
        Self {
            data: HashMap::new(),
        }
    }
}
