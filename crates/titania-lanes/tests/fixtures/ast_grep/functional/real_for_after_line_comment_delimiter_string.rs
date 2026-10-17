
pub fn first_item(items: &[i32]) -> Option<i32> {
    let marker = "//"; for item in items { let _ = marker; return Some(*item); }
    None
}
