use std::{iter::Peekable, str::Chars};

pub(super) fn compact(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub(super) fn remove_spaces(line: &str) -> String {
    line.chars().filter(|ch| !ch.is_whitespace()).collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ApiSourceLine {
    code: String,
}

impl ApiSourceLine {
    pub(super) fn parse(raw: &str, block_comment: &mut bool) -> Self {
        Self { code: strip_non_code(raw, block_comment).trim().to_owned() }
    }

    pub(super) fn code(&self) -> &str {
        self.code.as_str()
    }
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

fn start_comment_or_string(
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

enum StripAction {
    Skip,
    Stop,
    Space,
    Push(char),
}

fn strip_action(state: &mut StripState, ch: char, chars: &mut Peekable<Chars<'_>>) -> StripAction {
    if consume_block_comment(state, ch, chars) || consume_string(state, ch) {
        return StripAction::Skip;
    }
    if begins_line_comment(ch, chars) {
        return StripAction::Stop;
    }
    if start_comment_or_string(state, ch, chars) {
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
    while let Some(ch) = chars.next() {
        match strip_action(&mut state, ch, &mut chars) {
            StripAction::Skip => skip_action(),
            StripAction::Stop => break,
            StripAction::Space => code.push(' '),
            StripAction::Push(value) => code.push(value),
        }
    }
    *block_comment = state.block_comment;
    code
}

const fn skip_action() {}
