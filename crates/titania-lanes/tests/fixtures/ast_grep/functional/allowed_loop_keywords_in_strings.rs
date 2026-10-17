

pub fn process_all(items: &[i32]) -> Vec<i32> {
    items.iter().map(|x| x * 2).collect()
}

pub fn describe_loops() -> String {
    let example = "iterate over items and transform each one";
    format!("Example: {example}")
}
