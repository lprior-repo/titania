//! Registry-document model and fail-closed parser.
//!
//! Unknown fields are rejected, never ignored; the optional per-entry
//! `authority:` defaults to production when omitted.

use thiserror::Error;

/// Workspace-relative proof-obligation registry path.
pub const PROOF_OBLIGATIONS_PATH: &str = "contracts/proof_obligations.yaml";

/// Required production obligations that must stay registered as production.
///
/// A registry that drops one of these rows hides a required obligation and
/// fails, however healthy the remaining rows are.
pub const REQUIRED_PRODUCTION_OBLIGATIONS: &[&str] = &["verification/verus/receipt_schema.rs"];

/// Authority declared for one registered proof source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryAuthority {
    /// Counts toward production proof coverage when backed by genuine proof.
    Production,
    /// Explicitly non-authoritative fixture; accounted but never production.
    SmokeFixture,
}

/// One registered proof source with its declared authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    /// Workspace-relative source path; the stable obligation identity.
    pub path: String,
    /// Declared authority; defaults to production when the registry omits it.
    pub authority: RegistryAuthority,
}

/// Parsed proof-obligation registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registry {
    /// Registry schema version.
    pub version: u32,
    /// Registered proof sources in declaration order.
    pub targets: Vec<RegistryEntry>,
}

/// Typed registry-document parse failure.
#[derive(Debug, Error)]
pub enum RegistryError {
    /// No usable version line was found.
    #[error("registry is missing a `version:` line")]
    MissingVersion,
    /// A version line carries a missing, non-numeric, or zero value.
    #[error("registry has an invalid version on line {line}: `{value}`")]
    InvalidVersion {
        /// 1-indexed document line.
        line: u32,
        /// Raw version text.
        value: String,
    },
    /// No `targets:` section was declared.
    #[error("registry is missing a `targets:` section")]
    MissingTargets,
    /// A line matches no registry sentence.
    #[error("registry has an unexpected line {line}")]
    UnexpectedLine {
        /// 1-indexed document line.
        line: u32,
    },
    /// A `- path:` entry carries an empty path.
    #[error("registry has an empty path on line {line}")]
    EmptyPath {
        /// 1-indexed document line.
        line: u32,
    },
    /// A path is registered twice.
    #[error("registry registers `{path}` twice")]
    DuplicatePath {
        /// Duplicated workspace-relative path.
        path: String,
    },
    /// An `authority:` line carries an unknown authority value.
    #[error("registry has an invalid authority on line {line}: `{value}`")]
    InvalidAuthority {
        /// 1-indexed document line.
        line: u32,
        /// Raw authority text.
        value: String,
    },
    /// An `authority:` line has no preceding `- path:` entry to attach to, or
    /// the preceding entry already declares one.
    #[error("registry has a misplaced authority on line {line}")]
    MisplacedAuthority {
        /// 1-indexed document line.
        line: u32,
    },
}

/// Parse the proof-obligation registry document.
///
/// Unknown fields fail closed; the optional per-entry `authority:` defaults
/// to production when omitted.
///
/// # Errors
/// Returns [`RegistryError`] when the document is not exactly the registry
/// schema.
pub fn parse_registry(content: &str) -> Result<Registry, RegistryError> {
    let state = content
        .lines()
        .zip(1_u32..)
        .try_fold(ParseState::new(), |state, (line, number)| parse_step(state, line, number))?;
    state.finish()
}

/// One registry entry before its authority defaults to production.
struct RawEntry {
    path: String,
    authority: Option<RegistryAuthority>,
}

/// Functional registry-parser accumulator.
struct ParseState {
    version: Option<u32>,
    seen_targets: bool,
    entries: Vec<RawEntry>,
}

impl ParseState {
    /// Empty parser accumulator.
    const fn new() -> Self {
        Self { version: None, seen_targets: false, entries: Vec::new() }
    }

    /// Finish into a validated registry.
    ///
    /// # Errors
    /// Returns [`RegistryError`] for a missing version or a missing
    /// `targets:` section.
    fn finish(self) -> Result<Registry, RegistryError> {
        let version = self.version.ok_or(RegistryError::MissingVersion)?;
        require_targets_section(self.seen_targets)?;
        let targets =
            self.entries.into_iter().map(RawEntry::finalize).collect::<Vec<RegistryEntry>>();
        Ok(Registry { version, targets })
    }
}

/// Fail when no `targets:` section was declared.
///
/// # Errors
/// Returns [`RegistryError::MissingTargets`] when `seen` is false.
const fn require_targets_section(seen: bool) -> Result<(), RegistryError> {
    if seen { Ok(()) } else { Err(RegistryError::MissingTargets) }
}

impl RawEntry {
    /// Apply the production default to an entry without declared authority.
    fn finalize(self) -> RegistryEntry {
        let authority = self.authority.map_or(RegistryAuthority::Production, |authority| authority);
        RegistryEntry { path: self.path, authority }
    }
}

/// Fold one document line into the parser accumulator.
///
/// # Errors
/// Returns [`RegistryError`] when the line matches no registry sentence or
/// violates schema placement rules.
fn parse_step(state: ParseState, line: &str, number: u32) -> Result<ParseState, RegistryError> {
    let text = line.trim();
    if text.is_empty() || text.starts_with('#') {
        return Ok(state);
    }
    if text == "targets:" {
        return parse_targets_header(state, number);
    }
    if let Some(rest) = text.strip_prefix("version:") {
        return parse_version_line(state, rest, number);
    }
    if let Some(rest) = text.strip_prefix("- path:") {
        return parse_path_line(state, rest, number);
    }
    if let Some(rest) = text.strip_prefix("authority:") {
        return parse_authority_line(state, rest, number);
    }
    Err(RegistryError::UnexpectedLine { line: number })
}

/// Fold a `targets:` header into the parser accumulator.
///
/// # Errors
/// Returns [`RegistryError`] for a duplicated `targets:` header.
fn parse_targets_header(state: ParseState, number: u32) -> Result<ParseState, RegistryError> {
    if state.seen_targets {
        return Err(RegistryError::UnexpectedLine { line: number });
    }
    Ok(ParseState { version: state.version, seen_targets: true, entries: state.entries })
}

/// Fold a `version:` line into the parser accumulator.
///
/// # Errors
/// Returns [`RegistryError`] for a duplicated or invalid version value.
fn parse_version_line(
    state: ParseState,
    rest: &str,
    number: u32,
) -> Result<ParseState, RegistryError> {
    if state.version.is_some() {
        return Err(RegistryError::UnexpectedLine { line: number });
    }
    let value = rest.trim().to_owned();
    let version = match value.parse::<u32>() {
        Ok(version) if version > 0 => version,
        Ok(_) | Err(_) => {
            return Err(RegistryError::InvalidVersion { line: number, value });
        }
    };
    Ok(ParseState {
        version: Some(version),
        seen_targets: state.seen_targets,
        entries: state.entries,
    })
}

/// Fold a `- path:` entry into the parser accumulator.
///
/// # Errors
/// Returns [`RegistryError`] for entries outside `targets:`, empty paths,
/// and duplicated paths.
fn parse_path_line(
    state: ParseState,
    rest: &str,
    number: u32,
) -> Result<ParseState, RegistryError> {
    if !state.seen_targets {
        return Err(RegistryError::UnexpectedLine { line: number });
    }
    let path = rest.trim().to_owned();
    if path.is_empty() {
        return Err(RegistryError::EmptyPath { line: number });
    }
    if state.entries.iter().any(|entry| entry.path == path) {
        return Err(RegistryError::DuplicatePath { path });
    }
    let mut entries = state.entries;
    entries.push(RawEntry { path, authority: None });
    Ok(ParseState { version: state.version, seen_targets: state.seen_targets, entries })
}

/// Fold an `authority:` line into the preceding entry.
///
/// # Errors
/// Returns [`RegistryError`] for unknown authorities and for lines with no
/// preceding entry to attach to.
fn parse_authority_line(
    state: ParseState,
    rest: &str,
    number: u32,
) -> Result<ParseState, RegistryError> {
    let value = rest.trim();
    let authority = match value {
        "production" => RegistryAuthority::Production,
        "smoke-fixture" => RegistryAuthority::SmokeFixture,
        _ => {
            return Err(RegistryError::InvalidAuthority { line: number, value: value.to_owned() });
        }
    };
    let mut entries = state.entries;
    let attachable = entries.last().is_some_and(|entry| entry.authority.is_none());
    if !state.seen_targets || !attachable {
        return Err(RegistryError::MisplacedAuthority { line: number });
    }
    if let Some(entry) = entries.last_mut() {
        entry.authority = Some(authority);
    }
    Ok(ParseState { version: state.version, seen_targets: state.seen_targets, entries })
}
