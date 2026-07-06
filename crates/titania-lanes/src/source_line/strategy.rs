use std::convert::identity;

use super::SourceLineState;

pub(super) enum ParseStrategy {
    Borrowed,
    Parsed,
}

impl ParseStrategy {
    pub(super) fn for_line(raw: &str, state: SourceLineState) -> Self {
        match state {
            SourceLineState::Code => can_borrow(raw).then_some(Self::Borrowed),
            SourceLineState::BlockComment | SourceLineState::RawString { .. } => None,
        }
        .map_or(Self::Parsed, identity)
    }
}

/// The line needs transformation iff it contains a string-literal
/// opener (including raw strings), a comment starter, or a line comment.
/// A bare `/` (division) is harmless.
fn can_borrow(raw: &str) -> bool {
    !raw.contains('"')
        && !raw.contains("//")
        && !raw.contains("/*")
        && !raw.contains("r\"")
        && !raw.contains("r#\"")
        && !raw.contains("r##\"")
        && !raw.contains("br\"")
        && !raw.contains("br#\"")
        && !raw.contains("br##\"")
}
