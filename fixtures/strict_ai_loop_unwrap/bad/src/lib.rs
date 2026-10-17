/// Bad function: uses an unbounded `loop` retry and `.unwrap()` — violates the
/// bounded-execution contract and functional lint rules.
///
/// # Panics
///
/// Panics when any element of `items` is `None`, because `.unwrap()` is called on
/// `Option<i32>` values during the unbounded retry.
#[must_use]
pub fn bad_function(items: Vec<Option<i32>>) -> Vec<i32> {
    let mut result = Vec::new();
    let mut index = 0;
    loop {
        if index >= items.len() {
            break;
        }
        let value = items[index].clone().unwrap();
        result.push(value);
        index += 1;
    }
    result
}
