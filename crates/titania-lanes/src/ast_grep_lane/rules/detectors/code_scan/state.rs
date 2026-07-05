//! Scanner state enums.

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum ScanMode {
    #[default]
    Code,
    Text,
    RawStart,
    RawText,
    RawEnd,
    LineComment,
    BlockComment,
}

impl ScanMode {
    pub(super) const fn finish_line(self) -> Self {
        match self {
            Self::LineComment => Self::Code,
            mode => mode,
        }
    }

    pub(super) const fn finish_raw(self) -> Self {
        match self {
            Self::RawEnd => Self::RawText,
            mode => mode,
        }
    }

    pub(super) const fn raw_hash_seen_after_finish(self, seen: u8) -> u8 {
        match self {
            Self::RawEnd => 0,
            _ => seen,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum CodePending {
    #[default]
    None,
    Slash,
    RawPrefix,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum TextEscape {
    #[default]
    Normal,
    Escaped,
}

impl TextEscape {
    pub(super) const fn next(self, ch: char) -> Self {
        match (self, ch) {
            (Self::Normal, '\\') => Self::Escaped,
            _ => Self::Normal,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) enum BlockMark {
    #[default]
    None,
    Star,
    Slash,
}
