

pub fn process_all(items: &[i32]) -> Vec<i32> {
    items.iter().map(|item| item * 2).collect()
}
