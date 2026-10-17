
#[allow(dead_code)]
pub fn unused_but_allowed() -> i32 {
    42
}

#[allow(clippy::manual_map)]
pub fn manual_map(x: Option<i32>) -> i32 {
    match x {
        Some(v) => v,
        None => 0,
    }
}
