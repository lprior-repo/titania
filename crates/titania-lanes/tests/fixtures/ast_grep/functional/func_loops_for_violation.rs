// Fixture: triggers FUNC_LOOPS_FOR
// Imperative for-loop in production source — should be an iterator pipeline.

pub fn process_items(items: &[i32]) -> i32 {
    let mut sum = 0;
    for item in items {
        sum += item;
    }
    sum
}

pub fn collect_names(records: &[(String, i32)]) -> Vec<String> {
    let mut names = Vec::new();
    for (name, _age) in records {
        names.push(name.clone());
    }
    names
}
