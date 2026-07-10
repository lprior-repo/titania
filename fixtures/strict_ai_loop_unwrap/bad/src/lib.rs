/// Bad function: uses a for-loop and `.unwrap()` — violates functional lint rules.
///
/// # Panics
///
/// Panics when any element of `items` is `None`, because `.unwrap()` is called on
/// `Option<i32>` values during iteration.
#[must_use]
pub fn bad_function(items: Vec<Option<i32>>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        let value = item.unwrap();
        result.push(value);
    }
    result
}
