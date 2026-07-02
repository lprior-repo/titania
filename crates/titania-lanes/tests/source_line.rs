#![allow(clippy::pedantic, clippy::nursery, clippy::default_numeric_fallback)]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
#![allow(clippy::indexing_slicing)]

use titania_lanes::SourceLine;

fn parse_lines(text: &str) -> Vec<SourceLine> {
    let mut block_comment = false;
    text.lines().map(|line| SourceLine::parse(line, &mut block_comment)).collect()
}

#[test]
fn line_comment_is_skipped() {
    let lines = parse_lines("// hello\nlet x = 1;");
    assert!(lines[0].is_non_code());
    assert_eq!(lines[1].code(), "let x = 1;");
}

#[test]
fn block_comment_within_one_line_is_skipped() {
    let lines = parse_lines("/* foo */ let x = 1;");
    let code = lines[0].code();
    assert!(!code.contains("foo"));
    assert!(code.contains("let"));
}

#[test]
fn block_comment_spans_multiple_lines() {
    let mut block_comment = false;
    let _line1 = SourceLine::parse("/* spans", &mut block_comment);
    assert!(block_comment, "block_comment should remain open");
    let line2 = SourceLine::parse("more lines */ let x = 1;", &mut block_comment);
    assert!(!block_comment, "block_comment should be closed");
    assert!(line2.code().contains("let x = 1;"));
}

#[test]
fn string_literal_contents_are_blanked_out() {
    let lines = parse_lines("let s = \"assert!\";");
    let code = lines[0].code();
    assert!(!code.contains("assert!"));
    assert!(code.contains("let s = "));
}

#[test]
fn escaped_quote_in_string_does_not_close() {
    let lines = parse_lines(r#"let s = "a\"b";"#);
    let code = lines[0].code();
    assert!(code.starts_with("let s = "));
    assert!(code.ends_with(';'));
    assert!(!code.contains(r#"a\"b"#));
}

#[test]
fn line_with_raw_string_blanks_contents() {
    let lines = parse_lines(r##"let s = r#"hello"#;"##);
    let code = lines[0].code();
    assert!(code.contains("let s = "), "surrounding code must survive");
    assert!(!code.contains("hello"), "raw string contents must be blanked");
    assert!(!code.contains("r#"), "raw-string prefix r# must not leak");
}

#[test]
fn line_with_byte_string_blanks_contents() {
    let lines = parse_lines(r#"let b = b"world";"#);
    let code = lines[0].code();
    assert!(code.contains("let b = "), "surrounding code must survive");
    assert!(!code.contains("world"), "byte string contents must be blanked");
    assert!(
        code.contains("let b =        ;"),
        "byte-string prefix and content should be blanked (got '{code}')"
    );
}

#[test]
fn line_with_two_hash_raw_string_blanks_contents() {
    let lines = parse_lines(r###"let s = r##"world"##;"###);
    let code = lines[0].code();
    assert!(code.contains("let s = "), "surrounding code must survive");
    assert!(!code.contains("world"), "two-hash raw string contents must be blanked");
    assert!(!code.contains("r##"), "two-hash raw-string prefix r## must not leak");
}

#[test]
fn line_with_unterminated_raw_string_keeps_code() {
    let lines = parse_lines(r#"let x = 1; r#"unterminated"#);
    let code = lines[0].code();
    assert!(code.contains("let x = 1;"), "code before raw string must survive");
    assert!(!code.contains("r#"), "raw-string prefix r# must not leak");
}
