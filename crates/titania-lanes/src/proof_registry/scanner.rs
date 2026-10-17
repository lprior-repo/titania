//! Single-pass scanner that skips comments and string literals.
//!
//! Words accumulate only in code mode, so proof tokens inside comments or
//! strings never count. Over-stripping around lifetime ticks fails loud as
//! proof-free; it can never manufacture a silent pass.

use super::classify::{WordAction, is_word_char};

/// Single-pass scanner mode.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Scan {
    /// Normal code; words accumulate here.
    Code(CodeSub),
    /// Comment content; skipped.
    Comment(CommentSub),
    /// String, character, or raw-string content; skipped.
    Text(TextSub),
}

/// Code-level scanner sub-state.
#[derive(Debug, Clone, Copy)]
pub(crate) enum CodeSub {
    /// Between tokens.
    Ready,
    /// Just saw `/`; `//`, `/*`, or division follows.
    SawSlash,
    /// Just saw `r` (plus `hashes` `#`) at a word start; a raw string,
    /// an identifier continuation, or a boundary follows.
    SawRaw {
        /// `#` characters seen after the `r`.
        hashes: u8,
    },
}

/// Comment scanner sub-state.
#[derive(Debug, Clone, Copy)]
pub(crate) enum CommentSub {
    /// Inside `//` until newline.
    Line,
    /// Inside `/*` with nesting depth.
    Block {
        /// Nesting depth, at least one.
        depth: u32,
    },
    /// Inside `/*` just after `*`; `/` would close one level.
    BlockStar {
        /// Nesting depth, at least one.
        depth: u32,
    },
    /// Inside `/*` just after `/`; `*` would nest one level deeper.
    BlockSlash {
        /// Nesting depth, at least one.
        depth: u32,
    },
}

/// String-literal scanner sub-state.
#[derive(Debug, Clone, Copy)]
pub(crate) enum TextSub {
    /// Inside `"..."`.
    Quoted {
        /// Previous character was an unescaped backslash.
        escaped: bool,
    },
    /// Inside `'...'` or a lifetime tick; bails out to code past short
    /// literals so lifetime annotations cannot swallow real code.
    Tick {
        /// Content characters consumed so far.
        length: u8,
        /// Previous character was an unescaped backslash.
        escaped: bool,
    },
    /// Inside `r#"..."#` with hash count.
    Raw {
        /// Opening hash count.
        hashes: u8,
    },
    /// Inside a raw string just after `"`; `#` characters may close it.
    RawEnd {
        /// Opening hash count.
        hashes: u8,
        /// Closing `#` characters matched so far.
        matched: u8,
    },
}

impl Scan {
    /// Scanner starting in normal code.
    pub(crate) const fn code() -> Self {
        Self::Code(CodeSub::Ready)
    }
}

/// Fold one character into the scanner, returning the word-buffer action.
pub(crate) const fn scan_step(mode: Scan, next: char) -> (Scan, WordAction) {
    match mode {
        Scan::Code(sub) => step_code(sub, next),
        Scan::Comment(sub) => (step_comment(sub, next), WordAction::None),
        Scan::Text(sub) => (step_text(sub, next), WordAction::None),
    }
}

/// Fold one character while in code.
const fn step_code(sub: CodeSub, next: char) -> (Scan, WordAction) {
    match sub {
        CodeSub::Ready => step_code_ready(next),
        CodeSub::SawSlash => step_code_slash(next),
        CodeSub::SawRaw { hashes } => step_code_raw(hashes, next),
    }
}

/// Enter the literal matching one quote character.
const fn text_entry(next: char) -> TextSub {
    if next == '"' {
        TextSub::Quoted { escaped: false }
    } else {
        TextSub::Tick { length: 0, escaped: false }
    }
}

/// Fold one character while between code tokens.
const fn step_code_ready(next: char) -> (Scan, WordAction) {
    match next {
        '/' => (Scan::Code(CodeSub::SawSlash), WordAction::Check),
        'r' => (Scan::Code(CodeSub::SawRaw { hashes: 0 }), WordAction::Push(next)),
        '"' | '\'' => (Scan::Text(text_entry(next)), WordAction::Check),
        _ if is_word_char(next) => (Scan::Code(CodeSub::Ready), WordAction::Push(next)),
        _ => (Scan::Code(CodeSub::Ready), WordAction::Check),
    }
}

/// Fold one character just after a code `/`.
const fn step_code_slash(next: char) -> (Scan, WordAction) {
    match next {
        '/' => (Scan::Comment(CommentSub::Line), WordAction::None),
        '*' => (Scan::Comment(CommentSub::Block { depth: 1 }), WordAction::None),
        'r' => (Scan::Code(CodeSub::SawRaw { hashes: 0 }), WordAction::Push(next)),
        '"' | '\'' => (Scan::Text(text_entry(next)), WordAction::None),
        _ if is_word_char(next) => (Scan::Code(CodeSub::Ready), WordAction::Push(next)),
        _ => (Scan::Code(CodeSub::Ready), WordAction::None),
    }
}

/// Fold one character after `r` plus tracked `#` characters.
const fn step_code_raw(hashes: u8, next: char) -> (Scan, WordAction) {
    match next {
        '"' => (Scan::Text(TextSub::Raw { hashes }), WordAction::Clear),
        '#' => {
            (Scan::Code(CodeSub::SawRaw { hashes: hashes.saturating_add(1) }), WordAction::Check)
        }
        _ if is_word_char(next) => (Scan::Code(CodeSub::Ready), WordAction::Push(next)),
        _ => (Scan::Code(CodeSub::Ready), WordAction::Check),
    }
}

/// Fold one character while inside a comment.
const fn step_comment(sub: CommentSub, next: char) -> Scan {
    match sub {
        CommentSub::Line => step_line_comment(next),
        CommentSub::Block { depth } => step_block_comment(depth, next),
        CommentSub::BlockStar { depth } => step_block_star(depth, next),
        CommentSub::BlockSlash { depth } => step_block_slash(depth, next),
    }
}

/// Fold one character while inside a `//` comment.
const fn step_line_comment(next: char) -> Scan {
    if next == '\n' { Scan::code() } else { Scan::Comment(CommentSub::Line) }
}

/// Fold one character while inside a `/*` comment.
const fn step_block_comment(depth: u32, next: char) -> Scan {
    match next {
        '*' => Scan::Comment(CommentSub::BlockStar { depth }),
        '/' => Scan::Comment(CommentSub::BlockSlash { depth }),
        _ => Scan::Comment(CommentSub::Block { depth }),
    }
}

/// Fold one character just after `*` inside a block comment.
const fn step_block_star(depth: u32, next: char) -> Scan {
    match next {
        '/' => close_block_level(depth),
        '*' => Scan::Comment(CommentSub::BlockStar { depth }),
        _ => Scan::Comment(CommentSub::Block { depth }),
    }
}

/// Close one block-comment level, returning to code at depth zero.
const fn close_block_level(depth: u32) -> Scan {
    let rest = depth.saturating_sub(1);
    if rest == 0 { Scan::code() } else { Scan::Comment(CommentSub::Block { depth: rest }) }
}

/// Fold one character just after `/` inside a block comment.
const fn step_block_slash(depth: u32, next: char) -> Scan {
    if next == '*' {
        Scan::Comment(CommentSub::Block { depth: depth.saturating_add(1) })
    } else {
        Scan::Comment(CommentSub::Block { depth })
    }
}

/// Fold one character while inside a string-like literal.
const fn step_text(sub: TextSub, next: char) -> Scan {
    match sub {
        TextSub::Quoted { escaped } => step_quoted(escaped, next),
        TextSub::Tick { length, escaped } => step_tick(length, escaped, next),
        TextSub::Raw { hashes } => step_raw_string(hashes, next),
        TextSub::RawEnd { hashes, matched } => step_raw_end(hashes, matched, next),
    }
}

/// Fold one character while inside a `"..."` literal.
const fn step_quoted(escaped: bool, next: char) -> Scan {
    if next == '"' && !escaped {
        Scan::code()
    } else {
        Scan::Text(TextSub::Quoted { escaped: next == '\\' && !escaped })
    }
}

/// Fold one character while inside a `'...'` literal or lifetime tick.
const fn step_tick(length: u8, escaped: bool, next: char) -> Scan {
    match next {
        '\'' if !escaped => Scan::code(),
        _ => extend_tick(length, escaped, next),
    }
}

/// Fold a non-closing character while inside a tick literal.
const fn extend_tick(length: u8, escaped: bool, next: char) -> Scan {
    let length = length.saturating_add(1);
    if length > 2 {
        Scan::code()
    } else {
        Scan::Text(TextSub::Tick { length, escaped: next == '\\' && !escaped })
    }
}

/// Fold one character while inside a raw-string body.
const fn step_raw_string(hashes: u8, next: char) -> Scan {
    match next {
        '"' if hashes == 0 => Scan::code(),
        '"' => Scan::Text(TextSub::RawEnd { hashes, matched: 0 }),
        _ => Scan::Text(TextSub::Raw { hashes }),
    }
}

/// Fold one character just after `"` inside a raw string.
const fn step_raw_end(hashes: u8, matched: u8, next: char) -> Scan {
    match next {
        '#' => close_raw_hash(hashes, matched),
        _ => Scan::Text(TextSub::Raw { hashes }),
    }
}

/// Fold one `#` that may close a raw string.
const fn close_raw_hash(hashes: u8, matched: u8) -> Scan {
    let matched = matched.saturating_add(1);
    if matched == hashes { Scan::code() } else { Scan::Text(TextSub::RawEnd { hashes, matched }) }
}
