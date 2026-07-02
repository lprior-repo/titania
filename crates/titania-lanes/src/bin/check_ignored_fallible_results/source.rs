use std::{iter::Peekable, str::Chars};

#[derive(Clone, Debug, Eq, PartialEq)]
enum LineKind {
    NonCode,
    Signature,
    Expression,
}

/// Source line after comments and signatures are classified for scanning.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLine {
    code: String,
    kind: LineKind,
}

impl SourceLine {
    /// Strip non-code segments and classify a raw source line.
    pub fn parse(raw: &str, block_comment: &mut bool) -> Self {
        let code = strip_non_code(raw, block_comment);
        let trimmed = code.trim().to_owned();
        let kind = classify_kind(&trimmed);
        Self { code: trimmed, kind }
    }

    /// Return the stripped and trimmed code segment.
    #[must_use]
    pub fn code(&self) -> &str {
        self.code.as_str()
    }

    /// Report whether this line is a function signature.
    #[must_use]
    pub fn is_signature(&self) -> bool {
        self.kind == LineKind::Signature
    }

    /// Report whether this line is a source expression worth scanning.
    #[must_use]
    pub fn is_code_expression(&self) -> bool {
        self.kind == LineKind::Expression
    }
}

fn classify_kind(trimmed: &str) -> LineKind {
    if trimmed.is_empty() {
        LineKind::NonCode
    } else if is_signature_line(trimmed) {
        LineKind::Signature
    } else {
        LineKind::Expression
    }
}

fn is_signature_line(trimmed: &str) -> bool {
    let looks_like_fn = [
        "fn ",
        "pub fn ",
        "pub fn ",
        "pub fn ",
        "async fn ",
        "pub async fn ",
        "const fn ",
        "pub const fn ",
    ]
    .iter()
    .any(|prefix| trimmed.starts_with(prefix));
    looks_like_fn && trimmed.contains('(')
}

#[derive(Clone, Copy)]
struct StripState {
    block_comment: bool,
    in_string: bool,
    escaped: bool,
}

impl StripState {
    const fn new(block_comment: bool) -> Self {
        Self { block_comment, in_string: false, escaped: false }
    }
}

fn consume_block_comment(
    state: &mut StripState,
    ch: char,
    chars: &mut Peekable<Chars<'_>>,
) -> bool {
    if !state.block_comment {
        return false;
    }
    if ch == '*' && chars.peek().is_some_and(|next| *next == '/') {
        let _slash = chars.next();
        state.block_comment = false;
    }
    true
}

const fn consume_string(state: &mut StripState, ch: char) -> bool {
    if !state.in_string {
        return false;
    }
    if state.escaped {
        state.escaped = false;
        return true;
    }
    if ch == '\\' {
        state.escaped = true;
        return true;
    }
    if ch == '"' {
        state.in_string = false;
    }
    true
}

fn start_block_comment_or_string(
    state: &mut StripState,
    ch: char,
    chars: &mut Peekable<Chars<'_>>,
) -> bool {
    if ch == '/' && chars.peek().is_some_and(|next| *next == '*') {
        let _star = chars.next();
        state.block_comment = true;
        return true;
    }
    if ch == '"' {
        state.in_string = true;
        return true;
    }
    false
}

#[derive(Clone, Copy)]
enum StripAction {
    Skip,
    Stop,
    Space,
    Push(char),
}

fn apply_strip_action(action: StripAction, code: &mut String) -> bool {
    match action {
        StripAction::Skip => true,
        StripAction::Stop => false,
        StripAction::Space => {
            code.push(' ');
            true
        }
        StripAction::Push(value) => {
            code.push(value);
            true
        }
    }
}

fn consume_next_char(
    state: &mut StripState,
    chars: &mut Peekable<Chars<'_>>,
    code: &mut String,
) -> bool {
    let Some(ch) = chars.next() else {
        return false;
    };
    apply_strip_action(strip_action(state, ch, chars), code)
}

fn strip_action(state: &mut StripState, ch: char, chars: &mut Peekable<Chars<'_>>) -> StripAction {
    if consume_block_comment(state, ch, chars) || consume_string(state, ch) {
        return StripAction::Skip;
    }
    if begins_line_comment(ch, chars) {
        return StripAction::Stop;
    }
    if start_block_comment_or_string(state, ch, chars) {
        return StripAction::Space;
    }
    StripAction::Push(ch)
}

fn begins_line_comment(ch: char, chars: &mut Peekable<Chars<'_>>) -> bool {
    ch == '/' && chars.peek().is_some_and(|next| *next == '/')
}

fn strip_non_code(raw: &str, block_comment: &mut bool) -> String {
    let mut code = String::with_capacity(raw.len());
    let mut chars = raw.chars().peekable();
    let mut state = StripState::new(*block_comment);
    while consume_next_char(&mut state, &mut chars, &mut code) {}
    *block_comment = state.block_comment;
    code
}
