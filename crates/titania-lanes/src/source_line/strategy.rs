use std::convert::identity;

pub(super) enum ParseStrategy {
    Borrowed,
    Parsed,
}

impl ParseStrategy {
    pub(super) fn for_line(raw: &str, block_comment: bool) -> Self {
        (!block_comment && can_borrow(raw)).then_some(Self::Borrowed).map_or(Self::Parsed, identity)
    }
}

/// The line needs transformation iff it contains a string-literal
/// opener or a comment starter. A bare `/` (division) is harmless.
fn can_borrow(raw: &str) -> bool {
    !raw.contains('"') && !raw.contains("//") && !raw.contains("/*")
}
