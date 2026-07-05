//! Source cleanup helpers for the embedded ast-grep lane.

use crate::source_line::SourceLine;

/// Strip comments and blank string literals while preserving line numbers.
pub(super) fn clean_source(source: &str) -> String {
    let mut block_comment = false;
    source
        .lines()
        .map(|raw| SourceLine::parse(raw, &mut block_comment).code().to_owned())
        .collect::<Vec<_>>()
        .join("\n")
}
