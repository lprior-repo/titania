use std::{sync::Mutex};

pub fn counter() -> Mutex<i32> {
    Mutex::new(0)
}
