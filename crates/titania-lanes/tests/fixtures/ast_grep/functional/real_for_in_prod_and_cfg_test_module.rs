// Fixture: for-loop in production code AND a for-loop in #[cfg(test)] mod tests.
// The production for-loop MUST trigger FUNC_LOOPS_FOR; the test for-loop must NOT.

pub fn bad_iteration(items: &[i32]) -> i32 {
    let mut sum = 0;
    for item in items {
        sum += item;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_bad_iteration() {
        let items = [1, 2, 3];
        let mut sum = 0;
        for item in &items {
            sum += item;
        }
        assert_eq!(sum, 6);
    }
}
