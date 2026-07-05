//! Raw-string state transitions.

use super::{CodePending, LineScan, ScanMode};

enum RawDelimiter {
    Complete,
    Partial,
}

impl RawDelimiter {
    fn for_counts(seen: u8, expected: u8) -> Self {
        match seen.cmp(&expected) {
            std::cmp::Ordering::Equal => Self::Complete,
            _ => Self::Partial,
        }
    }
}

impl LineScan {
    pub(super) fn enter_raw_start(mut self) -> Self {
        self.mode = ScanMode::RawStart;
        self.raw_hashes = self.raw_hashes.checked_add(1).map_or(self.raw_hashes, |next| next);
        self
    }

    pub(super) const fn enter_raw_text(mut self) -> Self {
        self.mode = ScanMode::RawText;
        self.raw_hash_seen = 0;
        self
    }

    pub(super) fn accept_raw_start(self, ch: char) -> Self {
        match ch {
            '#' => self.add_raw_hash(),
            '"' => self.enter_raw_text(),
            _ => self.cancel_raw_start().accept_visible_code(ch),
        }
    }

    pub(super) const fn accept_raw_text(self, ch: char) -> Self {
        match (ch, self.raw_hashes) {
            ('"', 0) => self.exit_raw_text(),
            ('"', _) => self.enter_raw_end(),
            _ => self,
        }
    }

    pub(super) fn accept_raw_end(self, ch: char) -> Self {
        match ch {
            '#' => self.accept_raw_end_hash(),
            _ => self.return_to_raw_text(),
        }
    }

    pub(super) const fn finish_raw_end(mut self) -> Self {
        let mode = self.mode;
        self.mode = mode.finish_raw();
        self.raw_hash_seen = mode.raw_hash_seen_after_finish(self.raw_hash_seen);
        self
    }

    fn add_raw_hash(mut self) -> Self {
        self.raw_hashes = self.raw_hashes.checked_add(1).map_or(self.raw_hashes, |next| next);
        self
    }

    const fn enter_raw_end(mut self) -> Self {
        self.mode = ScanMode::RawEnd;
        self.raw_hash_seen = 0;
        self
    }

    fn accept_raw_end_hash(mut self) -> Self {
        self.raw_hash_seen =
            self.raw_hash_seen.checked_add(1).map_or(self.raw_hash_seen, |next| next);
        match RawDelimiter::for_counts(self.raw_hash_seen, self.raw_hashes) {
            RawDelimiter::Complete => self.exit_raw_text(),
            RawDelimiter::Partial => self,
        }
    }

    const fn return_to_raw_text(mut self) -> Self {
        self.mode = ScanMode::RawText;
        self.raw_hash_seen = 0;
        self
    }

    const fn exit_raw_text(mut self) -> Self {
        self.mode = ScanMode::Code;
        self.raw_hashes = 0;
        self.raw_hash_seen = 0;
        self
    }

    fn cancel_raw_start(mut self) -> Self {
        self.mode = ScanMode::Code;
        self.code.push('r');
        self.code.extend(std::iter::repeat_n('#', usize::from(self.raw_hashes)));
        self.raw_hashes = 0;
        self.raw_hash_seen = 0;
        self
    }

    pub(super) fn emit_pending_raw_prefix_then(mut self, ch: char) -> Self {
        self.pending = CodePending::None;
        self.code.push('r');
        self.accept_visible_code(ch)
    }

    pub(super) fn emit_pending_raw_prefix(mut self) -> Self {
        self.code.push('r');
        self.pending = CodePending::None;
        self
    }
}
