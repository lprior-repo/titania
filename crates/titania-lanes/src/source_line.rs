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

use std::{iter::Peekable, str::Chars};

/// A source line after stripping comments and string contents.
///
/// `Code` carries the surviving runes (with string contents replaced
/// by spaces so the byte count and column positions are preserved).
/// `NonCode` means the whole line was a comment or string; the
/// scanner can skip it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceLine {
    Code(String),
    NonCode,
}

impl SourceLine {
    /// Tokenize one line. `block_comment` is the carry-over flag from
    /// the previous line: `true` if we are inside a `/* … */` block
    /// that has not yet been closed. The function updates it in place.
    #[must_use]
    pub fn parse(raw: &str, block_comment: &mut bool) -> Self {
        let parser = SourceLineParser::new(raw, *block_comment);
        let parsed = parser.parse();
        *block_comment = parsed.block_comment;
        parsed.line
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
            Self::Code(code) => code.as_str(),
            Self::NonCode => "",
        }
    }
}

struct ParsedLine {
    line: SourceLine,
    block_comment: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StringMode {
    Normal { escaped: bool },
    Raw { hashes: u8 },
}

struct SourceLineParser<'a> {
    chars: Peekable<Chars<'a>>,
    code: String,
    block_comment: bool,
    string_mode: Option<StringMode>,
}

impl<'a> SourceLineParser<'a> {
    fn new(raw: &'a str, block_comment: bool) -> Self {
        Self {
            chars: raw.chars().peekable(),
            code: String::with_capacity(raw.len()),
            block_comment,
            string_mode: None,
        }
    }

    fn parse(mut self) -> ParsedLine {
        while let Some(ch) = self.chars.next() {
            if self.consume_block_comment(ch) || self.consume_string(ch) {
                continue;
            }
            if self.starts_line_comment(ch) {
                break;
            }
            if self.starts_block_comment(ch) {
                continue;
            }
            self.consume_code(ch);
        }
        self.finish()
    }

    fn consume_block_comment(&mut self, ch: char) -> bool {
        if !self.block_comment {
            return false;
        }
        if ch == '*' && self.chars.peek().is_some_and(|next| *next == '/') {
            let _slash = self.chars.next();
            self.block_comment = false;
        }
        true
    }

    fn consume_string(&mut self, ch: char) -> bool {
        match self.string_mode {
            None => false,
            Some(StringMode::Normal { escaped }) => {
                self.consume_normal_string_char(ch, escaped);
                true
            }
            Some(StringMode::Raw { hashes }) => {
                self.consume_raw_string_char(ch, hashes);
                true
            }
        }
    }

    fn starts_line_comment(&mut self, ch: char) -> bool {
        ch == '/' && self.chars.peek().is_some_and(|next| *next == '/')
    }

    fn starts_block_comment(&mut self, ch: char) -> bool {
        if ch != '/' || self.chars.peek().is_none_or(|next| *next != '*') {
            return false;
        }
        let _star = self.chars.next();
        self.block_comment = true;
        true
    }

    fn consume_code(&mut self, ch: char) {
        match ch {
            '"' => self.start_normal_string_with_prefix_width(1),
            'b' => {
                if !self.try_start_byte_string() {
                    self.code.push(ch);
                }
            }
            'r' => {
                if !self.try_start_raw_string() {
                    self.code.push(ch);
                }
            }
            _ => self.code.push(ch),
        }
    }

    fn consume_normal_string_char(&mut self, ch: char, escaped: bool) {
        if escaped {
            self.string_mode = Some(StringMode::Normal { escaped: false });
            self.code.push(' ');
        } else if ch == '\\' {
            self.string_mode = Some(StringMode::Normal { escaped: true });
            self.code.push(' ');
        } else if ch == '"' {
            self.string_mode = None;
        } else {
            self.code.push(' ');
        }
    }

    fn consume_raw_string_char(&mut self, ch: char, hashes: u8) {
        if ch == '"' && self.raw_terminator_follows(hashes) {
            self.consume_hashes(hashes);
            self.string_mode = None;
        } else {
            self.code.push(' ');
        }
    }

    fn try_start_byte_string(&mut self) -> bool {
        if self.chars.peek().is_some_and(|next| *next == '"') {
            let _quote = self.chars.next();
            self.start_normal_string_with_prefix_width(2);
            true
        } else if self.byte_raw_string_follows() {
            let _raw_prefix = self.chars.next();
            self.try_start_raw_string_after_r(2)
        } else {
            false
        }
    }

    fn try_start_raw_string(&mut self) -> bool {
        self.try_start_raw_string_after_r(1)
    }

    fn try_start_raw_string_after_r(&mut self, prefix_width: u8) -> bool {
        match raw_hash_count_before_quote(self.chars.clone()) {
            Some(hashes) => {
                self.push_spaces(prefix_width);
                self.consume_hashes_as_spaces(hashes);
                let _quote = self.chars.next();
                self.code.push(' ');
                self.string_mode = Some(StringMode::Raw { hashes });
                true
            }
            None => false,
        }
    }

    fn byte_raw_string_follows(&self) -> bool {
        let mut probe = self.chars.clone();
        match probe.next() {
            Some('r') => raw_hash_count_before_quote(probe).is_some(),
            _ => false,
        }
    }

    fn raw_terminator_follows(&self, hashes: u8) -> bool {
        let mut probe = self.chars.clone();
        (0..hashes).all(|_| {
            let matched = probe.peek().is_some_and(|next| *next == '#');
            if matched {
                let _hash = probe.next();
            }
            matched
        })
    }

    fn start_normal_string_with_prefix_width(&mut self, prefix_width: u8) {
        self.push_spaces(prefix_width);
        self.string_mode = Some(StringMode::Normal { escaped: false });
    }

    fn consume_hashes(&mut self, hashes: u8) {
        for _ in 0..hashes {
            let _hash = self.chars.next();
        }
    }

    fn consume_hashes_as_spaces(&mut self, hashes: u8) {
        for _ in 0..hashes {
            let _hash = self.chars.next();
            self.code.push(' ');
        }
    }

    fn push_spaces(&mut self, count: u8) {
        for _ in 0..count {
            self.code.push(' ');
        }
    }

    fn finish(self) -> ParsedLine {
        let line = if self.code.trim().is_empty() {
            SourceLine::NonCode
        } else {
            SourceLine::Code(self.code)
        };
        ParsedLine { line, block_comment: self.block_comment }
    }
}

fn raw_hash_count_before_quote(mut chars: Peekable<Chars<'_>>) -> Option<u8> {
    let mut hashes = 0_u8;
    while chars.peek().is_some_and(|next| *next == '#') {
        let _hash = chars.next();
        hashes = hashes.checked_add(1)?;
    }
    chars.next().is_some_and(|next| next == '"').then_some(hashes)
}
