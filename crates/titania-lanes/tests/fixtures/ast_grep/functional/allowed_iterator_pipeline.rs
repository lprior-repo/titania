// Fixture: allowed — uses iterator pipeline instead of for-loop.

pub fn process_items(items: &[i32]) -> i32 {
    items.iter().sum()
}

pub fn collect_names(records: &[(String, i32)]) -> Vec<String> {
    records.iter().map(|(name, _age)| name.clone()).collect()
}

pub fn countdown(n: u64) -> Vec<u64> {
    (1..=n).rev().collect()
}
