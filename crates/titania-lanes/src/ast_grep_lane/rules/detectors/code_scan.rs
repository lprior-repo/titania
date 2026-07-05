//! One-pass code scanner for text-backed detectors.

mod block;
mod raw;
mod state;

use state::{BlockMark, CodePending, ScanMode, TextEscape};

pub(super) fn detect_code_line(source: &str, predicate: fn(&str) -> bool) -> bool {
    source
        .lines()
        .try_fold(CodeScan::default(), |state, line| state.accept(line, predicate))
        .is_break()
}

#[derive(Default)]
struct CodeScan {
    mode: ScanMode,
    escape: TextEscape,
    block_depth: u8,
    block_mark: BlockMark,
    raw_hashes: u8,
    raw_hash_seen: u8,
}

impl CodeScan {
    fn accept(self, line: &str, predicate: fn(&str) -> bool) -> std::ops::ControlFlow<(), Self> {
        let stripped = LineScan::from(self).scan(line).finish();
        predicate(&stripped.code)
            .then_some(std::ops::ControlFlow::Break(()))
            .map_or(std::ops::ControlFlow::Continue(stripped.next), |flow| flow)
    }
}

struct StrippedLine {
    next: CodeScan,
    code: String,
}

struct LineScan {
    mode: ScanMode,
    escape: TextEscape,
    block_depth: u8,
    block_mark: BlockMark,
    pending: CodePending,
    raw_hashes: u8,
    raw_hash_seen: u8,
    code: String,
}

impl From<CodeScan> for LineScan {
    fn from(scan: CodeScan) -> Self {
        Self {
            mode: scan.mode,
            escape: scan.escape,
            block_depth: scan.block_depth,
            block_mark: scan.block_mark,
            pending: CodePending::None,
            raw_hashes: scan.raw_hashes,
            raw_hash_seen: scan.raw_hash_seen,
            code: String::new(),
        }
    }
}

impl LineScan {
    fn scan(self, line: &str) -> Self {
        line.chars().fold(self, Self::accept)
    }

    fn accept(self, ch: char) -> Self {
        match self.mode {
            ScanMode::Code => self.accept_code(ch),
            ScanMode::Text => self.accept_text(ch),
            ScanMode::RawStart => self.accept_raw_start(ch),
            ScanMode::RawText => self.accept_raw_text(ch),
            ScanMode::RawEnd => self.accept_raw_end(ch),
            ScanMode::LineComment => self,
            ScanMode::BlockComment => self.accept_block_comment(ch),
        }
    }

    fn accept_code(self, ch: char) -> Self {
        match self.pending {
            CodePending::Slash => self.accept_after_slash(ch),
            CodePending::RawPrefix => self.accept_after_raw_prefix(ch),
            CodePending::None => self.accept_visible_code(ch),
        }
    }

    fn accept_after_slash(mut self, ch: char) -> Self {
        self.pending = CodePending::None;
        match ch {
            '/' => self.enter_line_comment(),
            '*' => self.enter_block_comment(),
            _ => self.emit_pending_slash_then(ch),
        }
    }

    fn accept_after_raw_prefix(mut self, ch: char) -> Self {
        self.pending = CodePending::None;
        match ch {
            '"' => self.enter_raw_text(),
            '#' => self.enter_raw_start(),
            _ => self.emit_pending_raw_prefix_then(ch),
        }
    }

    pub(super) fn accept_visible_code(mut self, ch: char) -> Self {
        match ch {
            '/' => self.pending = CodePending::Slash,
            '"' => self.enter_text(),
            'r' => self.pending = CodePending::RawPrefix,
            _ => self.code.push(ch),
        }
        self
    }

    const fn enter_text(&mut self) {
        self.mode = ScanMode::Text;
        self.escape = TextEscape::Normal;
    }

    const fn accept_text(mut self, ch: char) -> Self {
        self.mode = match (self.escape, ch) {
            (TextEscape::Normal, '"') => ScanMode::Code,
            _ => ScanMode::Text,
        };
        self.escape = self.escape.next(ch);
        self
    }

    fn emit_pending_slash_then(mut self, ch: char) -> Self {
        self.code.push('/');
        self.accept_visible_code(ch)
    }

    fn finish(self) -> StrippedLine {
        self.finish_pending_raw_prefix()
            .finish_pending_slash()
            .finish_line_comment()
            .finish_raw_end()
            .stripped()
    }

    fn finish_pending_raw_prefix(self) -> Self {
        match self.pending {
            CodePending::RawPrefix => self.emit_pending_raw_prefix(),
            _ => self,
        }
    }

    fn finish_pending_slash(self) -> Self {
        match self.pending {
            CodePending::Slash => self.emit_pending_slash(),
            _ => self,
        }
    }

    fn emit_pending_slash(mut self) -> Self {
        self.code.push('/');
        self.pending = CodePending::None;
        self
    }

    fn stripped(self) -> StrippedLine {
        StrippedLine {
            next: CodeScan {
                mode: self.mode,
                escape: self.escape,
                block_depth: self.block_depth,
                block_mark: self.block_mark,
                raw_hashes: self.raw_hashes,
                raw_hash_seen: self.raw_hash_seen,
            },
            code: self.code,
        }
    }
}
