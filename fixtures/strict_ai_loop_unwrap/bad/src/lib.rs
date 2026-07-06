/// Bad function: uses a for-loop and .unwrap() — violates functional lint rules.
pub fn bad_function(items: Vec<Option<i32>>) -> Vec<i32> {
    let mut result = Vec::new();
    for item in items {
        let value = item.unwrap();
        result.push(value);
    }
    result
}
