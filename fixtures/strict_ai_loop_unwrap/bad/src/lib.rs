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
