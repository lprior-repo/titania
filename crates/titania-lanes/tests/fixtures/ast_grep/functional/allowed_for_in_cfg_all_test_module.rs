// Fixture: for-loop inside #[cfg(all(test, feature = "debug"))] mod tests —
// must NOT trigger FUNC_LOOPS_FOR. The cfg(all(test, ...)) form is also a
// test module per v1-spec §9.10.

pub fn clean_map(items: &[i32]) -> Vec<i32> {
    items.iter().copied().map(|x| x.saturating_add(1)).collect()
}

#[cfg(all(test, feature = "debug"))]
mod debug_tests {
    use super::*;

    #[test]
    fn iterates_with_debug() {
        let items = [1, 2, 3];
        let mut acc = 0;
        for item in &items {
            acc += item;
        }
        assert_eq!(acc, 6);
    }
}
