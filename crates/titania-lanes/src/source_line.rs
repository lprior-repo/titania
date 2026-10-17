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

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SourceLineState {
    #[default]
    Code,
    BlockComment,
    RawString {
        hashes: u8,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceLine<'a> {
    Code(Cow<'a, str>),
    NonCode,
}

impl<'a> SourceLine<'a> {
    #[must_use]
    pub fn parse(raw: &'a str, state: &mut SourceLineState) -> Self {
        match ParseStrategy::for_line(raw, *state) {
            ParseStrategy::Borrowed => finish_borrowed(raw),
            ParseStrategy::Parsed => parser::parse_owned(raw, state),
        }
    }

    #[must_use]
    pub const fn is_non_code(&self) -> bool {
        matches!(self, Self::NonCode)
    }

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
        let line = &lines[0];
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
        let input = String::from("let s = r#\"unwrap()\"#;");
        let lines = parse_lines(&input);
        let code = lines[0].code();
        assert!(!code.contains("r#"), "raw-string prefix leaked: {code}");
        assert!(code.contains("let s = "), "code prefix lost: {code}");
    }

    #[test]
    fn byte_raw_string_contents_are_blanked_out() {
        let input = String::from("let b = br#\"panic!\"#;");
        let lines = parse_lines(&input);
        let code = lines[0].code();
        assert!(!code.contains("br#"), "byte raw-string prefix leaked: {code}");
        assert!(code.contains("let b = "), "code prefix lost: {code}");
    }

    #[test]
    fn raw_string_with_multiple_hash_delimiters() {
        let input = String::from("let s = r##\"simple\"##;");
        let lines = parse_lines(&input);
        let code = lines[0].code();
        assert!(!code.contains("r##"), "multi-hash raw-string prefix leaked: {code}");
        assert!(code.contains("let s = "), "code prefix lost: {code}");
    }

    #[test]
    fn raw_string_with_hash_content() {
        let input = String::from("let s = r##\"hash content\"##;");
        let lines = parse_lines(&input);
        let code = lines[0].code();
        assert!(!code.contains("r##"), "raw string prefix leaked: {code}");
        assert!(code.contains("let s = "), "code prefix lost: {code}");
    }

    #[test]
    fn mixed_regular_and_raw_strings_both_blanked() {
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
        let input = String::from("const MSG = r#\"assert!(false)\"#;");
        let lines = parse_lines(&input);
        let code = lines[0].code();
        assert!(!code.contains("r#"), "raw-string prefix leaked: {code}");
        assert!(!code.contains("assert!"), "raw-string content leaked: {code}");
    }
    #[test]
    fn multi_line_raw_string_blank_contents_on_both_lines() {
        let input = "let s = r#\"\nsafe unwrap()\n\"#;";
        let lines = parse_lines(input);
        assert_eq!(lines.len(), 3, "expected three lines");
        let line0 = lines[0].code();
        assert!(line0.contains("let s = "), "code prefix lost: {line0}");
        let line1 = lines[1].code();
        assert!(
            line1.is_empty() || line1.trim().is_empty(),
            "raw string body leaked on line 1: \"{line1}\" — forbidden tokens inside raw strings must be blanked"
        );
        let line2 = lines[2].code();
        assert!(
            !line2.contains("unwrap()"),
            "forbidden token leaked from raw string body: {line2}"
        );
    }

    #[test]
    fn multi_line_raw_string_with_forbidden_token_on_close_line() {
        let input = "let s = r#\"\nsome content\n\"#;\nlet x = unwrap();";
        let lines = parse_lines(input);
        assert_eq!(lines.len(), 4, "expected four lines");
        let body = lines[1].code();
        assert!(body.trim().is_empty(), "raw string body leaked: \"{body}\"");
        let code3 = lines[3].code();
        assert!(code3.contains("unwrap()"), "code after raw string close must survive: {code3}");
    }

    #[test]
    fn zero_hash_raw_string_r_quote_blank() {
        let input = "let s = r\"unwrap()\";";
        let lines = parse_lines(input);
        let code = lines[0].code();
        assert!(code.contains("let s = "), "code prefix lost: {code}");
        assert!(code.ends_with(';'), "trailing semicolon lost: {code}");
        let after_prefix = match code.strip_prefix("let s = ") {
            Some(s) => s,
            None => code,
        };
        assert!(
            after_prefix.chars().all(|c| c.is_whitespace() || c == ';'),
            "raw string r prefix leaked as code: {code}"
        );
    }

    #[test]
    fn raw_string_with_interior_quote_and_hash() {
        let input = "let s = r#\"has \"quote\" inside\"#;";
        let lines = parse_lines(input);
        let code = lines[0].code();
        assert!(code.contains("let s = "), "code prefix lost: {code}");
        assert!(
            !code.contains("has \"quote\" inside"),
            "interior quotes leaked from raw string: {code}"
        );
    }

    #[test]
    fn multi_line_raw_string_closes_with_code_on_same_line() {
        let input = "let s = r#\"\ncontent\n\"#; real_call();";
        let lines = parse_lines(input);
        assert_eq!(lines.len(), 3, "expected three lines");
        assert!(
            lines[1].code().trim().is_empty(),
            "raw string body leaked: \"{}\"",
            lines[1].code()
        );
        let code2 = lines[2].code();
        assert!(code2.contains("real_call()"), "code after raw string close must survive: {code2}");
    }
}
