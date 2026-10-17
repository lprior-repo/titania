// Fixture: unbounded retry inside #[cfg(all(test, feature = "debug"))]
// mod tests — must NOT trigger BOUNDED_EXECUTION_UNBOUNDED_LOOP. The
// cfg(all(test, ...)) form is also a test module per v1-spec §9.10.

pub fn clean_map(items: &[i32]) -> Vec<i32> {
    items.iter().copied().map(|x| x.saturating_add(1)).collect()
}

#[cfg(all(test, feature = "debug"))]
mod debug_tests {
    #[test]
    fn retries_with_debug() {
        let mut attempts = 0;
        while attempts < 3 {
            attempts += 1;
        }
        assert_eq!(attempts, 3);
    }
}
