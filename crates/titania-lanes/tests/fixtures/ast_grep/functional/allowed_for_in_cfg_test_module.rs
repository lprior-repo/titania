// Fixture: for-loop inside #[cfg(test)] mod tests — must NOT trigger FUNC_LOOPS_FOR.
// Inline test modules in src/ files are exempt per v1-spec §9.10.

pub fn clean_pipeline(items: &[i32]) -> i32 {
    items.iter().copied().sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sums_correctly() {
        let items = [1, 2, 3];
        let mut sum = 0;
        for item in &items {
            sum += item;
        }
        assert_eq!(sum, 6);
    }
}
