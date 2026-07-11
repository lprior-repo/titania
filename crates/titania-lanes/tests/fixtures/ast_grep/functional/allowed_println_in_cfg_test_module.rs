// Fixture: println! inside #[cfg(test)] mod tests — must NOT trigger FUNC_PRINT_STDOUT.
// Inline test modules in src/ files are exempt per v1-spec §9.10.

pub fn compute(x: i32) -> i32 {
    x.saturating_add(1)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_print() {
        println!("debug: {}", compute(1));
        assert_eq!(compute(1), 2);
    }
}
