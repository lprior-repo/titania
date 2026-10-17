// Fixture: allowed — bounded `for` iteration over finite input with `?`.
// Bound::FiniteInput with the input length as evidence; the typed failure
// stays observable through `?`. No unbounded retry or iterator source.

pub fn total(items: &[i32]) -> Result<i32, &'static str> {
    let mut sum = 0;
    for value in items {
        sum += process(*value)?;
    }
    Ok(sum)
}

fn process(value: i32) -> Result<i32, &'static str> {
    if value < 0 {
        return Err("negative input");
    }
    Ok(value)
}

pub fn drain_with_while_let(items: &mut Vec<i32>) -> i32 {
    let mut sum = 0;
    while let Some(value) = items.pop() {
        sum += value;
    }
    sum
}
