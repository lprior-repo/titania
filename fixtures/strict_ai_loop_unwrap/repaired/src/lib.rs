/// Repaired function: uses iterator pipeline with flatten — no for-loop, no unwrap().
pub fn good_function(items: Vec<Option<i32>>) -> Vec<i32> {
    items.into_iter().flatten().collect()
}
