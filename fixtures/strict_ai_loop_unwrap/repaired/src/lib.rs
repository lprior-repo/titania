#[must_use]
pub fn good_function(items: Vec<Option<i32>>) -> Vec<i32> {
    items.into_iter().flatten().collect()
}
