use std::{iter::Peekable, str::Chars};

use super::{SourceLine, SourceLineState, finish_owned};

pub(super) fn parse_owned<'a>(raw: &'a str, state: &mut SourceLineState) -> SourceLine<'a> {
    let parser = SourceLineParser::new(raw, *state);
    let result = parser.parse();
    *state = result.state;
    result.line
}

struct ParseResult<'a> {
    line: SourceLine<'a>,
    state: SourceLineState,
}

struct SourceLineParser<'a> {
    chars: Peekable<Chars<'a>>,
    code: String,
    state: InternalState,
}

#[derive(Clone, Copy)]
enum InternalState {
    Code,
    BlockComment,
    String(StringMode),
    RawString(u8),
}

impl InternalState {
    const fn from_source(state: SourceLineState) -> Self {
        match state {
            SourceLineState::Code => Self::Code,
            SourceLineState::BlockComment => Self::BlockComment,
            SourceLineState::RawString { hashes } => Self::RawString(hashes),
        }
    }

    const fn to_source(self) -> SourceLineState {
        match self {
            Self::BlockComment => SourceLineState::BlockComment,
            Self::RawString(hashes) => SourceLineState::RawString { hashes },
            Self::Code | Self::String(_) => SourceLineState::Code,
        }
    }
}

impl<'a> SourceLineParser<'a> {
    fn new(raw: &'a str, state: SourceLineState) -> Self {
        Self {
            chars: raw.chars().peekable(),
            code: String::with_capacity(raw.len()),
            state: InternalState::from_source(state),
        }
    }

    fn parse(mut self) -> ParseResult<'a> {
        let _consumed = std::iter::from_fn(|| self.consume_next()).all(ScanStep::keep_going);
        self.finish()
    }

    fn consume_next(&mut self) -> Option<ScanStep> {
        self.chars.next().map(|ch| self.consume_char(ch))
    }

    fn consume_char(&mut self, ch: char) -> ScanStep {
        match self.state {
            InternalState::Code => self.consume_code(ch),
            InternalState::BlockComment => self.consume_block_comment(ch),
            InternalState::String(mode) => self.consume_string(ch, mode),
            InternalState::RawString(hash_count) => self.consume_raw_string(ch, hash_count),
        }
    }

    fn consume_block_comment(&mut self, ch: char) -> ScanStep {
        let _closed = self.block_comment_closes(ch).then(|| self.close_block_comment());
        ScanStep::Continue
    }

    fn block_comment_closes(&mut self, ch: char) -> bool {
        ch == '*' && self.chars.peek().is_some_and(|next| *next == '/')
    }

    fn close_block_comment(&mut self) {
        let _slash = self.chars.next();
        self.state = InternalState::Code;
    }

    const fn consume_string(&mut self, ch: char, mode: StringMode) -> ScanStep {
        StringToken::read(ch, mode).apply(self)
    }

    fn consume_code(&mut self, ch: char) -> ScanStep {
        self.try_enter_raw_string(ch).map_or_else(
            || CodeToken::read(ch, self.chars.peek()).apply(self),
            |()| ScanStep::Continue,
        )
    }

    fn try_enter_raw_string(&mut self, first: char) -> Option<()> {
        let mut probe = self.chars.clone();
        let hash_count = Self::parse_raw_prefix(first, &mut probe)?;
        self.chars = probe;
        self.code.push(' ');
        self.state = InternalState::RawString(hash_count);
        Some(())
    }

    fn parse_raw_prefix(first: char, chars: &mut Peekable<Chars<'a>>) -> Option<u8> {
        match first {
            'r' => Self::parse_raw_after_r(chars),
            'b' => Self::parse_byte_raw(chars),
            _ => None,
        }
    }

    fn parse_byte_raw(chars: &mut Peekable<Chars<'a>>) -> Option<u8> {
        let _raw_marker = chars.next_if_eq(&'r')?;
        Self::parse_raw_after_r(chars)
    }

    fn parse_raw_after_r(chars: &mut Peekable<Chars<'a>>) -> Option<u8> {
        let hash_count = Self::consume_hashes(chars);
        chars.next_if_eq(&'"').map(|_| hash_count)
    }

    fn consume_hashes(chars: &mut Peekable<Chars<'a>>) -> u8 {
        std::iter::from_fn(|| chars.next_if_eq(&'#')).fold(0u8, |count, _| count.saturating_add(1))
    }

    fn consume_raw_string(&mut self, ch: char, expected_hashes: u8) -> ScanStep {
        let _closed = (ch == '"').then(|| self.close_raw_string_if_match(expected_hashes));
        ScanStep::Continue
    }

    fn close_raw_string_if_match(&mut self, expected_hashes: u8) {
        let mut probe = self.chars.clone();
        let _closed = Self::consume_expected_hashes(&mut probe, expected_hashes)
            .then(|| self.close_raw_string(probe));
    }

    fn consume_expected_hashes(chars: &mut Peekable<Chars<'a>>, expected_hashes: u8) -> bool {
        (0..expected_hashes).try_fold((), |(), _| chars.next_if_eq(&'#').map(|_| ())).is_some()
    }

    fn close_raw_string(&mut self, next_chars: Peekable<Chars<'a>>) {
        self.chars = next_chars;
        self.code.push(' ');
        self.state = InternalState::Code;
    }

    fn start_block_comment(&mut self) -> ScanStep {
        let _star = self.chars.next();
        self.state = InternalState::BlockComment;
        ScanStep::Continue
    }

    fn start_string(&mut self) -> ScanStep {
        self.state = InternalState::String(StringMode::Normal);
        self.code.push(' ');
        ScanStep::Continue
    }

    const fn continue_string(&mut self, mode: StringMode) -> ScanStep {
        self.state = InternalState::String(mode);
        ScanStep::Continue
    }

    const fn end_string(&mut self) -> ScanStep {
        self.state = InternalState::Code;
        ScanStep::Continue
    }

    fn push_code(&mut self, ch: char) -> ScanStep {
        self.code.push(ch);
        ScanStep::Continue
    }

    fn finish(self) -> ParseResult<'a> {
        ParseResult { line: finish_owned(self.code), state: self.state.to_source() }
    }
}

#[derive(Clone, Copy)]
enum StringMode {
    Normal,
    Escaped,
}

enum ScanStep {
    Continue,
    Stop,
}

impl ScanStep {
    const fn keep_going(self) -> bool {
        matches!(self, Self::Continue)
    }
}

enum CodeToken {
    CodeChar(char),
    StringStart,
    BlockCommentStart,
    LineCommentStart,
}

impl CodeToken {
    const fn read(ch: char, next: Option<&char>) -> Self {
        match ch {
            '/' => Self::from_slash(SlashLookahead::read(next)),
            '"' => Self::StringStart,
            _ => Self::CodeChar(ch),
        }
    }

    const fn from_slash(lookahead: SlashLookahead) -> Self {
        match lookahead {
            SlashLookahead::Slash => Self::LineCommentStart,
            SlashLookahead::Star => Self::BlockCommentStart,
            SlashLookahead::Other => Self::CodeChar('/'),
        }
    }

    fn apply(self, parser: &mut SourceLineParser<'_>) -> ScanStep {
        match self {
            Self::CodeChar(ch) => parser.push_code(ch),
            Self::StringStart => parser.start_string(),
            Self::BlockCommentStart => parser.start_block_comment(),
            Self::LineCommentStart => ScanStep::Stop,
        }
    }
}

#[derive(Clone, Copy)]
enum SlashLookahead {
    Slash,
    Star,
    Other,
}

impl SlashLookahead {
    const fn read(next: Option<&char>) -> Self {
        match next.copied() {
            Some('/') => Self::Slash,
            Some('*') => Self::Star,
            Some(_) | None => Self::Other,
        }
    }
}

enum StringToken {
    Body,
    EscapeStart,
    EscapedChar,
    End,
}

impl StringToken {
    const fn read(ch: char, mode: StringMode) -> Self {
        match (mode, ch) {
            (StringMode::Escaped, _) => Self::EscapedChar,
            (StringMode::Normal, '\\') => Self::EscapeStart,
            (StringMode::Normal, '"') => Self::End,
            (StringMode::Normal, _) => Self::Body,
        }
    }

    const fn apply(self, parser: &mut SourceLineParser<'_>) -> ScanStep {
        match self {
            Self::Body | Self::EscapedChar => parser.continue_string(StringMode::Normal),
            Self::EscapeStart => parser.continue_string(StringMode::Escaped),
            Self::End => parser.end_string(),
        }
    }
}
