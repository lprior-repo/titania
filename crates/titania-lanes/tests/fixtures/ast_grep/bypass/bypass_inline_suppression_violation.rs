// Fixture: triggers BYPASS_INLINE_SUPPRESSION
// Inline ast-grep-ignore and sg-ignore comments suppress structural checks.

// ast-grep-ignore: allow dead code
pub fn ignored_by_ast_grep() -> i32 {
    1
}

// sg-ignore
pub fn ignored_by_sg() -> i32 {
    2
}

// ast-grep-ignore: allow unwrap
pub fn uses_unsafe_unwrap() -> String {
    Some("data".to_string()).unwrap()
}
