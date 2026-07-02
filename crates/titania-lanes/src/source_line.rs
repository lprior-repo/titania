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
    /// Tokenize one line. `block_comment` is the carry-over flag from
    /// the previous line: `true` if we are inside a `/* … */` block
    /// that has not yet been closed. The function updates it in place.
    #[must_use]
    pub fn parse(raw: &'a str, block_comment: &mut bool) -> Self {
        // Fast path: when not inside a spanning block comment and the
        // line contains no string literal or comment opener, no
        // transformation is possible — borrow the input slice directly
        // and skip the per-char buffer allocation.
        match ParseStrategy::for_line(raw, *block_comment) {
            ParseStrategy::Borrowed => finish_borrowed(raw),
            ParseStrategy::Parsed => parser::parse_owned(raw, block_comment),
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
    use super::SourceLine;

    fn parse_lines(text: &str) -> Vec<SourceLine<'_>> {
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
        let mut block_comment = false;
        drop(SourceLine::parse("/* spans", &mut block_comment));
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
        // The closing `"` after `b` ends the string; the literal
        // contents between the quotes are blanked but the
        // surrounding code survives.
        assert!(code.starts_with("let s = "));
        assert!(code.ends_with(';'));
        assert!(!code.contains(r#"a\"b"#));
    }

    #[test]
    fn plain_code_line_is_borrowed_without_allocation() {
        let mut block_comment = false;
        let line = SourceLine::parse("let x = 1;", &mut block_comment);
        assert!(matches!(line.code(), "let x = 1;"));
        assert!(!block_comment);
    }
}
