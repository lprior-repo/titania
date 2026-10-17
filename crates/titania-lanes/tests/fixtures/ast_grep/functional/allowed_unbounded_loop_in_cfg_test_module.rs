// Fixture: unbounded retry inside #[cfg(test)] mod tests — must NOT trigger
// BOUNDED_EXECUTION_UNBOUNDED_LOOP. Inline test modules in src/ files are
// exempt per v1-spec §9.10.

pub fn clean_pipeline(items: &[i32]) -> i32 {
    items.iter().copied().sum()
}

#[cfg(test)]
mod tests {
    #[test]
    fn retries_with_backoff() {
        let mut attempts = 0;
        loop {
            attempts += 1;
            if attempts >= 3 {
                break;
            }
        }
        assert_eq!(attempts, 3);
    }
}
