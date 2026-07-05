// Fixture: real imperative for-loop — should trigger FUNC_LOOPS_FOR.

pub fn sum_items(items: &[i32]) -> i32 {
    let mut total = 0;
    for item in items {
        total += item;
    }
    total
}

pub fn collect_names(records: &[(String, i32)]) -> Vec<String> {
    let mut names = Vec::new();
    for (name, _age) in records {
        names.push(name.clone());
    }
    names
}
