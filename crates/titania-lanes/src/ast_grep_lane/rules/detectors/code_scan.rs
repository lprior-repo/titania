mod block;
mod raw;
mod state;

use state::{BlockMark, CodePending, ScanMode, TextEscape};

pub(super) fn code_only_source(source: &str) -> String {
    source.split('\n').fold(CodeOnly::default(), CodeOnly::accept).code
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

#[derive(Default)]
struct CodeOnly {
    scan: CodeScan,
    code: String,
}

impl CodeOnly {
    fn accept(mut self, line: &str) -> Self {
        let stripped = LineScan::from(self.scan).scan(line).finish();
        self.scan = stripped.next;
        self.code.push_str(&stripped.code);
        self.code.push('\n');
        self
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
            ScanMode::CharLiteral => self.accept_char_literal(ch),
            ScanMode::RawStart => self.accept_raw_start(ch),
            ScanMode::RawText => self.accept_raw_text(ch),
            ScanMode::RawEnd => self.accept_raw_end(ch),
            ScanMode::LineComment => self.blanked(ch),
            ScanMode::BlockComment => self.accept_block_comment(ch),
        }
    }

    fn blank(&mut self, ch: char) {
        self.code.extend(std::iter::repeat_n(' ', ch.len_utf8()));
    }

    fn blanked(mut self, ch: char) -> Self {
        self.blank(ch);
        self
    }

    fn accept_code(self, ch: char) -> Self {
        match self.pending {
            CodePending::Slash => self.accept_after_slash(ch),
            CodePending::RawPrefix => self.accept_after_raw_prefix(ch),
            CodePending::QuoteStart => self.accept_after_quote_start(ch),
            CodePending::QuoteChar(first) => self.accept_after_quote_char(first, ch),
            CodePending::None => self.accept_visible_code(ch),
        }
    }

    fn accept_after_slash(mut self, ch: char) -> Self {
        self.pending = CodePending::None;
        match ch {
            '/' => self.blanked('/').blanked(ch).enter_line_comment(),
            '*' => self.blanked('/').blanked(ch).enter_block_comment(),
            _ => self.emit_pending_slash_then(ch),
        }
    }

    fn accept_after_raw_prefix(mut self, ch: char) -> Self {
        self.pending = CodePending::None;
        match ch {
            '"' => self.blanked('r').blanked(ch).enter_raw_text(),
            '#' => self.blanked('r').blanked(ch).enter_raw_start(),
            _ => self.emit_pending_raw_prefix_then(ch),
        }
    }

    pub(super) fn accept_visible_code(mut self, ch: char) -> Self {
        match ch {
            '/' => self.pending = CodePending::Slash,
            '"' => self.enter_text_blanked(ch),
            'r' => self.pending = CodePending::RawPrefix,
            '\'' => self.pending = CodePending::QuoteStart,
            _ => self.code.push(ch),
        }
        self
    }

    fn accept_after_quote_start(mut self, ch: char) -> Self {
        self.pending = CodePending::None;
        match ch {
            '\\' => self.blanked('\'').blanked(ch).enter_char_literal(TextEscape::Escaped),
            '\'' => self.blanked('\'').blanked(ch),
            _ => self.defer_quote_char(ch),
        }
    }

    const fn defer_quote_char(mut self, ch: char) -> Self {
        self.pending = CodePending::QuoteChar(ch);
        self
    }

    fn accept_after_quote_char(mut self, first: char, ch: char) -> Self {
        self.pending = CodePending::None;
        match ch {
            '\'' => self.blanked('\'').blanked(first).blanked(ch),
            _ => self.emit_lifetime_then(first, ch),
        }
    }

    fn emit_lifetime_then(mut self, first: char, ch: char) -> Self {
        self.code.push('\'');
        self.code.push(first);
        self.accept_visible_code(ch)
    }

    const fn enter_char_literal(mut self, escape: TextEscape) -> Self {
        self.mode = ScanMode::CharLiteral;
        self.escape = escape;
        self
    }

    fn accept_char_literal(mut self, ch: char) -> Self {
        self.blank(ch);
        match (self.escape, ch) {
            (TextEscape::Escaped, _) => self.consume_escaped_char(),
            (TextEscape::Normal, '\\') => self.enter_char_literal(TextEscape::Escaped),
            (TextEscape::Normal, '\'') => self.exit_char_literal(),
            (TextEscape::Normal, _) => self,
        }
    }

    const fn consume_escaped_char(mut self) -> Self {
        self.escape = TextEscape::Normal;
        self
    }

    const fn exit_char_literal(mut self) -> Self {
        self.mode = ScanMode::Code;
        self
    }

    const fn enter_text(&mut self) {
        self.mode = ScanMode::Text;
        self.escape = TextEscape::Normal;
    }

    fn enter_text_blanked(&mut self, ch: char) {
        self.blank(ch);
        self.enter_text();
    }

    fn accept_text(mut self, ch: char) -> Self {
        self.blank(ch);
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
            .finish_pending_quote()
            .finish_line_comment()
            .finish_raw_end()
            .stripped()
    }

    fn finish_pending_quote(self) -> Self {
        match self.pending {
            CodePending::QuoteStart => self.emit_pending_quote(),
            CodePending::QuoteChar(first) => self.emit_pending_quote_char(first),
            _ => self,
        }
    }

    fn emit_pending_quote(mut self) -> Self {
        self.code.push('\'');
        self.pending = CodePending::None;
        self
    }

    fn emit_pending_quote_char(mut self, first: char) -> Self {
        self.code.push('\'');
        self.code.push(first);
        self.pending = CodePending::None;
        self
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
