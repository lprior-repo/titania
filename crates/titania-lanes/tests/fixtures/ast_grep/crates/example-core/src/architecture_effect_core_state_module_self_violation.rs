use std::sync::{self};

pub fn counter() -> sync::Mutex<i32> {
    sync::Mutex::new(0)
}
