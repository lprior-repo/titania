//! Line and block comment transitions.

use super::{BlockMark, LineScan, ScanMode};

enum BlockClose {
    Outer,
    Nested,
}

impl BlockClose {
    const fn for_depth(depth: u8) -> Self {
        match depth {
            0 | 1 => Self::Outer,
            _ => Self::Nested,
        }
    }
}

impl LineScan {
    pub(super) fn accept_block_comment(self, ch: char) -> Self {
        match (self.block_mark, ch) {
            (BlockMark::Star, '/') => self.close_block_comment(),
            (BlockMark::Slash, '*') => self.open_nested_block_comment(),
            (_, '*') => self.mark_block(BlockMark::Star),
            (_, '/') => self.mark_block(BlockMark::Slash),
            _ => self.mark_block(BlockMark::None),
        }
    }

    pub(super) const fn enter_line_comment(mut self) -> Self {
        self.mode = ScanMode::LineComment;
        self
    }

    pub(super) const fn enter_block_comment(mut self) -> Self {
        self.mode = ScanMode::BlockComment;
        self.block_depth = 1;
        self.block_mark = BlockMark::None;
        self
    }

    pub(super) const fn finish_line_comment(mut self) -> Self {
        self.mode = self.mode.finish_line();
        self
    }

    fn close_block_comment(self) -> Self {
        match BlockClose::for_depth(self.block_depth) {
            BlockClose::Outer => self.exit_block_comment(),
            BlockClose::Nested => self.leave_nested_block_comment(),
        }
    }

    fn open_nested_block_comment(mut self) -> Self {
        self.block_depth = self.block_depth.checked_add(1).map_or(self.block_depth, |next| next);
        self.block_mark = BlockMark::None;
        self
    }

    fn leave_nested_block_comment(mut self) -> Self {
        self.block_depth = self.block_depth.checked_sub(1).map_or(1, |next| next);
        self.block_mark = BlockMark::None;
        self
    }

    const fn mark_block(mut self, mark: BlockMark) -> Self {
        self.block_mark = mark;
        self
    }

    const fn exit_block_comment(mut self) -> Self {
        self.mode = ScanMode::Code;
        self.block_depth = 0;
        self.block_mark = BlockMark::None;
        self
    }
}
