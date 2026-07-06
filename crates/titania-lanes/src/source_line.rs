//! Shared source-line tokenizer used by the scanner lanes.
//!
//! Each scanner needs the same primitive: walk a line of Rust source,
//! drop block/line comments, and replace string literals with spaces
//! so token searches don't fire on the *content* of strings. The
//! [`SourceLine::parse`] function does exactly that and remembers
//! whether a `/* … */` block comment is still open across lines (the
//! caller threads the `&mut bool` through the loop).
//!
//! Kept in the library crate so the panic-surface, forbidden-scan, and
//! future scan-style lanes share one well-tested lexer. See
//! `bin/forbidden_scan/lane.rs` and `bin/check_panic_surface.rs` for
//! the consumers.

#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(clippy::todo)]
#![deny(clippy::unimplemented)]
#![deny(clippy::indexing_slicing)]
#![deny(clippy::string_slice)]
#![deny(clippy::get_unwrap)]
#![deny(clippy::arithmetic_side_effects)]
#![deny(clippy::dbg_macro)]
#![deny(clippy::as_conversions)]
#![forbid(unsafe_code)]

mod parser;
mod strategy;

use std::borrow::Cow;

use strategy::ParseStrategy;

/// Typed carry state across lines for the shared source-line parser.
///
/// Tracks whether we are inside a spanning `/* … */` block comment or
/// a raw string literal (identified by its hash depth). Callers pass a
/// mutable reference to the state so the parser can update it in place
/// across lines.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SourceLineState {
    /// Normal code scanning (no spanning block comment or raw string).
    #[default]
    Code,
    /// Inside a `/* … */` block comment that has not yet been closed.
    BlockComment,
    /// Inside a raw-string literal with the given number of `#` hashes.
    RawString {
        /// Number of `#` characters that must appear after the closing `"` to end the string.
        hashes: u8,
    },
}

/// A source line after stripping comments and string contents.
///
/// `Code` carries the surviving runes (with string contents replaced
/// by spaces so the byte count and column positions are preserved). It
/// borrows the input line directly when no transformation was needed
/// (no string literals or comments present and not inside a spanning
/// block comment); otherwise it owns the transformed buffer. `NonCode`
/// means the whole line was a comment or string; the scanner can skip
/// it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceLine<'a> {
    /// Surviving code runes, with string contents replaced by spaces.
    Code(Cow<'a, str>),
    /// Whole line was a comment or string; the scanner can skip it.
    NonCode,
}

impl<'a> SourceLine<'a> {
    /// Tokenize one line. `state` carries the spanned context from
    /// the previous line (block comment or raw string). The function
    /// updates it in place so callers can thread it across all lines.
    #[must_use]
    pub fn parse(raw: &'a str, state: &mut SourceLineState) -> Self {
        match ParseStrategy::for_line(raw, *state) {
            ParseStrategy::Borrowed => finish_borrowed(raw),
            ParseStrategy::Parsed => parser::parse_owned(raw, state),
        }
    }

    /// True if the line was entirely comments or string contents.
    #[must_use]
    pub const fn is_non_code(&self) -> bool {
        matches!(self, Self::NonCode)
    }

    /// The surviving code bytes. Returns an empty slice for `NonCode`.
    #[must_use]
    pub fn code(&self) -> &str {
        match self {
            Self::Code(code) => code,
            Self::NonCode => "",
        }
    }
}

fn finish_borrowed(raw: &str) -> SourceLine<'_> {
    finish_line(raw.trim().is_empty(), Cow::Borrowed(raw))
}

pub(super) fn finish_owned<'a>(code: String) -> SourceLine<'a> {
    finish_line(code.trim().is_empty(), Cow::Owned(code))
}

fn finish_line(is_empty: bool, code: Cow<'_, str>) -> SourceLine<'_> {
    if is_empty { SourceLine::NonCode } else { SourceLine::Code(code) }
}

#[cfg(test)]
mod tests {
    use super::{SourceLine, SourceLineState};

    fn parse_lines(text: &str) -> Vec<SourceLine<'_>> {
        let mut state = SourceLineState::default();
        text.lines().map(|line| SourceLine::parse(line, &mut state)).collect()
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
        // The whole line collapses to whitespace when only the
        // comment was real code, but `is_non_code` here returns
        // false because the line still has visible code.
        let line = &lines[0];
        // `code()` returns the surviving runes with the comment
        // replaced by spaces.
        let code = line.code();
        assert!(!code.contains("foo"));
        assert!(code.contains("let"));
    }

    #[test]
    fn block_comment_spans_multiple_lines() {
        let mut state = SourceLineState::default();
        drop(SourceLine::parse("/* spans", &mut state));
        assert!(matches!(state, SourceLineState::BlockComment), "state should be BlockComment");
        let line2 = SourceLine::parse("more lines */ let x = 1;", &mut state);
        assert!(matches!(state, SourceLineState::Code), "state should be Code");
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
        // The closing `"` after `b` ends the string; the literal
        // contents between the quotes are blanked but the
        // surrounding code survives.
        assert!(code.starts_with("let s = "));
        assert!(code.ends_with(';'));
        assert!(!code.contains(r#"a\"b"#));
    }

    #[test]
    fn plain_code_line_is_borrowed_without_allocation() {
        let mut state = SourceLineState::default();
        let line = SourceLine::parse("let x = 1;", &mut state);
        assert!(matches!(line.code(), "let x = 1;"));
        assert!(matches!(state, SourceLineState::Code));
    }
    #[test]
    fn raw_string_literal_contents_are_blanked_out() {
        // `r#"..."#` must blank the ENTIRE raw string including r# prefix
        // and # suffix. Before the fix, r# and # survive as code.
        let input = String::from("let s = r#\"unwrap()\"#;");
        let lines = parse_lines(&input);
        let code = lines[0].code();
        assert!(!code.contains("r#"), "raw-string prefix leaked: {code}");
        assert!(code.contains("let s = "), "code prefix lost: {code}");
    }

    #[test]
    fn byte_raw_string_contents_are_blanked_out() {
        // `br#"..."#` must blank the entire raw string including br# prefix
        // and # suffix. Before the fix, br# and # survive as code.
        let input = String::from("let b = br#\"panic!\"#;");
        let lines = parse_lines(&input);
        let code = lines[0].code();
        assert!(!code.contains("br#"), "byte raw-string prefix leaked: {code}");
        assert!(code.contains("let b = "), "code prefix lost: {code}");
    }

    #[test]
    fn raw_string_with_multiple_hash_delimiters() {
        // r##"..."## must blank the entire raw string including r## prefix
        // and ## suffix. Before the fix, r## and ## survive as code.
        let input = String::from("let s = r##\"simple\"##;");
        let lines = parse_lines(&input);
        let code = lines[0].code();
        assert!(!code.contains("r##"), "multi-hash raw-string prefix leaked: {code}");
        assert!(code.contains("let s = "), "code prefix lost: {code}");
    }

    #[test]
    fn raw_string_with_hash_content() {
        // Raw string containing # should blank correctly.
        // The entire r##...## including # in content should be blanked.
        let input = String::from("let s = r##\"hash content\"##;");
        let lines = parse_lines(&input);
        let code = lines[0].code();
        assert!(!code.contains("r##"), "raw string prefix leaked: {code}");
        assert!(code.contains("let s = "), "code prefix lost: {code}");
    }

    #[test]
    fn mixed_regular_and_raw_strings_both_blanked() {
        // Both "..." and r#"..."# contents must be blanked.
        // The r# prefix and # suffix must also be blanked.
        let input = String::from("let x = \"unwrap()\"; let y = r#\"panic!\"#;");
        let lines = parse_lines(&input);
        let code = lines[0].code();
        assert!(!code.contains("r#"), "raw-string r# prefix leaked: {code}");
        assert!(!code.contains("unwrap()"), "regular string leaked: {code}");
        assert!(code.contains("let x = "), "first code prefix lost: {code}");
        assert!(code.contains("let y = "), "second code prefix lost: {code}");
    }

    #[test]
    fn raw_string_in_assignment_blanks_forbidden_token() {
        // Regression: r#"assert!(true)"# must blank entire raw string.
        // Before the fix, r# and # survive as code.
        let input = String::from("const MSG = r#\"assert!(false)\"#;");
        let lines = parse_lines(&input);
        let code = lines[0].code();
        assert!(!code.contains("r#"), "raw-string prefix leaked: {code}");
        assert!(!code.contains("assert!"), "raw-string content leaked: {code}");
    }
    #[test]
    fn multi_line_raw_string_blank_contents_on_both_lines() {
        // Raw string that opens on one line and closes on the next must blank
        // content on BOTH lines. The shared parser tracks RawString state
        // Before the fix, content on the line after the opening leaks through
        // as code (false positive for forbidden tokens).
        let input = "let s = r#\"\nsafe unwrap()\n\"#;";
        let lines = parse_lines(input);
        assert_eq!(lines.len(), 3, "expected three lines");
        // Line 0: `let s = r#"` — code prefix survives, opening `"` starts raw string
        let line0 = lines[0].code();
        assert!(line0.contains("let s = "), "code prefix lost: {line0}");
        // Line 1: `safe unwrap()` — inside raw string, must be fully blanked
        let line1 = lines[1].code();
        assert!(
            line1.is_empty() || line1.trim().is_empty(),
            "raw string body leaked on line 1: \"{line1}\" — forbidden tokens inside raw strings must be blanked"
        );
        // Line 2: `"#;` — closing delimiter blanked, semicolon preserved
        let line2 = lines[2].code();
        assert!(
            !line2.contains("unwrap()"),
            "forbidden token leaked from raw string body: {line2}"
        );
    }

    #[test]
    fn multi_line_raw_string_with_forbidden_token_on_close_line() {
        // When a raw string closes on its own line, the next line has real code
        // that must still be scanned. The raw string body on prior lines is blanked.
        let input = "let s = r#\"\nsome content\n\"#;\nlet x = unwrap();";
        let lines = parse_lines(input);
        assert_eq!(lines.len(), 4, "expected four lines");
        // Line 1: raw string body — blanked
        let body = lines[1].code();
        assert!(body.trim().is_empty(), "raw string body leaked: \"{body}\"");
        // Line 3: code after close — `unwrap()` must survive
        let code3 = lines[3].code();
        assert!(code3.contains("unwrap()"), "code after raw string close must survive: {code3}");
    }

    #[test]
    fn zero_hash_raw_string_r_quote_blank() {
        // r"..." (zero hash delimiters) must blank the string content.
        // The parser does not recognize bare r" as a raw-string start
        // (it requires r# or r##). So r appears as code and " starts
        // a regular string. The r prefix leaks as visible code.
        let input = "let s = r\"unwrap()\";";
        let lines = parse_lines(input);
        let code = lines[0].code();
        // The r prefix should NOT leak — it is part of the raw-string literal
        // and must be blanked along with the content.
        // Without the fix, code is "let s = r ;" — r leaks.
        assert!(code.contains("let s = "), "code prefix lost: {code}");
        assert!(code.ends_with(';'), "trailing semicolon lost: {code}");
        // After "let s = ", only spaces should follow (the blanked raw string),
        // not the literal 'r' character.
        let after_prefix = match code.strip_prefix("let s = ") {
            Some(s) => s,
            None => code,
        };
        // The raw string content (including r prefix) should be all spaces.
        assert!(
            after_prefix.chars().all(|c| c.is_whitespace() || c == ';'),
            "raw string r prefix leaked as code: {code}"
        );
    }

    #[test]
    fn raw_string_with_interior_quote_and_hash() {
        // r#"has "quote" inside"# — interior quotes in raw strings must not
        // prematurely close the string. The closing delimiter is #", not ".
        let input = "let s = r#\"has \"quote\" inside\"#;";
        let lines = parse_lines(input);
        let code = lines[0].code();
        assert!(code.contains("let s = "), "code prefix lost: {code}");
        // The quote characters inside the raw string must not be visible
        // We expect at most 1 quote: the semicolon-adjacent one if it leaked,
        // but ideally 0 quotes leaked from the raw string body.
        // The key assertion: the input content "has "quote" inside" must not
        // appear in the output.
        assert!(
            !code.contains("has \"quote\" inside"),
            "interior quotes leaked from raw string: {code}"
        );
    }

    #[test]
    fn multi_line_raw_string_closes_with_code_on_same_line() {
        // r#"...\n"#; real_call(); — the closing `"#;` appears at the start
        // of a line, and there is code after it on the SAME line.
        let input = "let s = r#\"\ncontent\n\"#; real_call();";
        let lines = parse_lines(input);
        assert_eq!(lines.len(), 3, "expected three lines");
        // Line 1: raw string body — blanked
        assert!(
            lines[1].code().trim().is_empty(),
            "raw string body leaked: \"{}\"",
            lines[1].code()
        );
        // Line 2: #"; real_call(); — closing delimiter + real code
        let code2 = lines[2].code();
        assert!(code2.contains("real_call()"), "code after raw string close must survive: {code2}");
    }
}
