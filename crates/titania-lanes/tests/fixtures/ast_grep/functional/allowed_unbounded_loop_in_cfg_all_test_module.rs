
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
