/// Collapse whitespace to single spaces.
pub(super) fn compact(line: &str) -> String {
    line.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Remove all whitespace characters.
pub(super) fn remove_spaces(line: &str) -> String {
    line.chars().filter(|ch| !ch.is_whitespace()).collect()
}

/// Source line after comments and strings are stripped.
/// Delegates to the shared `titania_lanes::SourceLine::parse`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct ApiSourceLine {
    code: String,
}

impl ApiSourceLine {
    /// Strip non-code segments. Delegates to the shared parser.
    pub(super) fn parse(raw: &str, state: &mut titania_lanes::SourceLineState) -> Self {
        let shared = titania_lanes::SourceLine::parse(raw, state);
        Self { code: shared.code().to_owned() }
    }

    pub(super) fn code(&self) -> &str {
        &self.code
    }
}
