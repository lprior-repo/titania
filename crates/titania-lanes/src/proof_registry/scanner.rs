use super::classify::{WordAction, is_word_char};

#[derive(Debug, Clone, Copy)]
pub(crate) enum Scan {
    Code(CodeSub),
    Comment(CommentSub),
    Text(TextSub),
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CodeSub {
    Ready,
    SawSlash,
    SawRaw { hashes: u8 },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum CommentSub {
    Line,
    Block { depth: u32 },
    BlockStar { depth: u32 },
    BlockSlash { depth: u32 },
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum TextSub {
    Quoted { escaped: bool },
    Tick { length: u8, escaped: bool },
    Raw { hashes: u8 },
    RawEnd { hashes: u8, matched: u8 },
}

impl Scan {
    pub(crate) const fn code() -> Self {
        Self::Code(CodeSub::Ready)
    }
}

pub(crate) const fn scan_step(mode: Scan, next: char) -> (Scan, WordAction) {
    match mode {
        Scan::Code(sub) => step_code(sub, next),
        Scan::Comment(sub) => (step_comment(sub, next), WordAction::None),
        Scan::Text(sub) => (step_text(sub, next), WordAction::None),
    }
}

const fn step_code(sub: CodeSub, next: char) -> (Scan, WordAction) {
    match sub {
        CodeSub::Ready => step_code_ready(next),
        CodeSub::SawSlash => step_code_slash(next),
        CodeSub::SawRaw { hashes } => step_code_raw(hashes, next),
    }
}

const fn text_entry(next: char) -> TextSub {
    if next == '"' {
        TextSub::Quoted { escaped: false }
    } else {
        TextSub::Tick { length: 0, escaped: false }
    }
}

const fn step_code_ready(next: char) -> (Scan, WordAction) {
    match next {
        '/' => (Scan::Code(CodeSub::SawSlash), WordAction::Check),
        'r' => (Scan::Code(CodeSub::SawRaw { hashes: 0 }), WordAction::Push(next)),
        '"' | '\'' => (Scan::Text(text_entry(next)), WordAction::Check),
        _ if is_word_char(next) => (Scan::Code(CodeSub::Ready), WordAction::Push(next)),
        _ => (Scan::Code(CodeSub::Ready), WordAction::Check),
    }
}

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

const fn step_comment(sub: CommentSub, next: char) -> Scan {
    match sub {
        CommentSub::Line => step_line_comment(next),
        CommentSub::Block { depth } => step_block_comment(depth, next),
        CommentSub::BlockStar { depth } => step_block_star(depth, next),
        CommentSub::BlockSlash { depth } => step_block_slash(depth, next),
    }
}

const fn step_line_comment(next: char) -> Scan {
    if next == '\n' { Scan::code() } else { Scan::Comment(CommentSub::Line) }
}

const fn step_block_comment(depth: u32, next: char) -> Scan {
    match next {
        '*' => Scan::Comment(CommentSub::BlockStar { depth }),
        '/' => Scan::Comment(CommentSub::BlockSlash { depth }),
        _ => Scan::Comment(CommentSub::Block { depth }),
    }
}

const fn step_block_star(depth: u32, next: char) -> Scan {
    match next {
        '/' => close_block_level(depth),
        '*' => Scan::Comment(CommentSub::BlockStar { depth }),
        _ => Scan::Comment(CommentSub::Block { depth }),
    }
}

const fn close_block_level(depth: u32) -> Scan {
    let rest = depth.saturating_sub(1);
    if rest == 0 { Scan::code() } else { Scan::Comment(CommentSub::Block { depth: rest }) }
}

const fn step_block_slash(depth: u32, next: char) -> Scan {
    if next == '*' {
        Scan::Comment(CommentSub::Block { depth: depth.saturating_add(1) })
    } else {
        Scan::Comment(CommentSub::Block { depth })
    }
}

const fn step_text(sub: TextSub, next: char) -> Scan {
    match sub {
        TextSub::Quoted { escaped } => step_quoted(escaped, next),
        TextSub::Tick { length, escaped } => step_tick(length, escaped, next),
        TextSub::Raw { hashes } => step_raw_string(hashes, next),
        TextSub::RawEnd { hashes, matched } => step_raw_end(hashes, matched, next),
    }
}

const fn step_quoted(escaped: bool, next: char) -> Scan {
    if next == '"' && !escaped {
        Scan::code()
    } else {
        Scan::Text(TextSub::Quoted { escaped: next == '\\' && !escaped })
    }
}

const fn step_tick(length: u8, escaped: bool, next: char) -> Scan {
    match next {
        '\'' if !escaped => Scan::code(),
        _ => extend_tick(length, escaped, next),
    }
}

const fn extend_tick(length: u8, escaped: bool, next: char) -> Scan {
    let length = length.saturating_add(1);
    if length > 2 {
        Scan::code()
    } else {
        Scan::Text(TextSub::Tick { length, escaped: next == '\\' && !escaped })
    }
}

const fn step_raw_string(hashes: u8, next: char) -> Scan {
    match next {
        '"' if hashes == 0 => Scan::code(),
        '"' => Scan::Text(TextSub::RawEnd { hashes, matched: 0 }),
        _ => Scan::Text(TextSub::Raw { hashes }),
    }
}

const fn step_raw_end(hashes: u8, matched: u8, next: char) -> Scan {
    match next {
        '#' => close_raw_hash(hashes, matched),
        _ => Scan::Text(TextSub::Raw { hashes }),
    }
}

const fn close_raw_hash(hashes: u8, matched: u8) -> Scan {
    let matched = matched.saturating_add(1);
    if matched == hashes { Scan::code() } else { Scan::Text(TextSub::RawEnd { hashes, matched }) }
}
