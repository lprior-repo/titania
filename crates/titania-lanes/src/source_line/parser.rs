use std::{convert::identity, iter::Peekable, str::Chars};

use super::{SourceLine, finish_owned};

pub(super) fn parse_owned<'a>(raw: &'a str, block_comment: &mut bool) -> SourceLine<'a> {
    let parser = SourceLineParser::new(raw, *block_comment);
    let parsed = parser.parse();
    *block_comment = parsed.block_comment;
    parsed.line
}

struct ParsedLine<'a> {
    line: SourceLine<'a>,
    block_comment: bool,
}

struct SourceLineParser<'a> {
    chars: Peekable<Chars<'a>>,
    code: String,
    state: ParseState,
}

impl<'a> SourceLineParser<'a> {
    fn new(raw: &'a str, block_comment: bool) -> Self {
        Self {
            chars: raw.chars().peekable(),
            code: String::with_capacity(raw.len()),
            state: ParseState::from_block_comment(block_comment),
        }
    }

    fn parse(mut self) -> ParsedLine<'a> {
        let _consumed = std::iter::from_fn(|| self.consume_next()).all(ScanStep::keep_going);
        self.finish()
    }

    /// Pull and process the next character. Returns `false` to stop the
    /// loop — either because the iterator is exhausted or because a
    /// line comment was found.
    fn consume_next(&mut self) -> Option<ScanStep> {
        self.chars.next().map(|ch| self.consume_char(ch))
    }

    /// Process one character. Returns `false` to stop (line comment found).
    fn consume_char(&mut self, ch: char) -> ScanStep {
        match self.state {
            ParseState::Code => self.consume_code(ch),
            ParseState::BlockComment => self.consume_block_comment(ch),
            ParseState::String(mode) => self.consume_string(ch, mode),
        }
    }

    fn consume_block_comment(&mut self, ch: char) -> ScanStep {
        BlockCommentToken::read(ch, self.chars.peek()).apply(self)
    }

    const fn consume_string(&mut self, ch: char, mode: StringMode) -> ScanStep {
        StringToken::read(ch, mode).apply(self)
    }

    fn consume_code(&mut self, ch: char) -> ScanStep {
        CodeToken::read(ch, self.chars.peek()).apply(self)
    }

    fn start_block_comment(&mut self) -> ScanStep {
        let _star = self.chars.next();
        self.state = ParseState::BlockComment;
        ScanStep::Continue
    }

    fn end_block_comment(&mut self) -> ScanStep {
        let _slash = self.chars.next();
        self.state = ParseState::Code;
        ScanStep::Continue
    }

    fn start_string(&mut self) -> ScanStep {
        self.state = ParseState::String(StringMode::Normal);
        self.code.push(' ');
        ScanStep::Continue
    }

    const fn continue_string(&mut self, mode: StringMode) -> ScanStep {
        self.state = ParseState::String(mode);
        ScanStep::Continue
    }

    const fn end_string(&mut self) -> ScanStep {
        self.state = ParseState::Code;
        ScanStep::Continue
    }

    fn push_code(&mut self, ch: char) -> ScanStep {
        self.code.push(ch);
        ScanStep::Continue
    }

    fn finish(self) -> ParsedLine<'a> {
        ParsedLine { line: finish_owned(self.code), block_comment: self.state.block_comment() }
    }
}

#[derive(Clone, Copy)]
enum ParseState {
    Code,
    BlockComment,
    String(StringMode),
}

impl ParseState {
    fn from_block_comment(block_comment: bool) -> Self {
        block_comment.then_some(Self::BlockComment).map_or(Self::Code, identity)
    }

    const fn block_comment(self) -> bool {
        matches!(self, Self::BlockComment)
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

enum BlockCommentToken {
    End,
    Body,
}

impl BlockCommentToken {
    fn read(ch: char, next: Option<&char>) -> Self {
        Self::from(ch == '*' && next.is_some_and(|next| *next == '/'))
    }

    fn apply(self, parser: &mut SourceLineParser<'_>) -> ScanStep {
        match self {
            Self::End => parser.end_block_comment(),
            Self::Body => ScanStep::Continue,
        }
    }
}

impl From<bool> for BlockCommentToken {
    fn from(is_end: bool) -> Self {
        is_end.then_some(Self::End).map_or(Self::Body, identity)
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
