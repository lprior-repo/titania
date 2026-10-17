
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
