use thiserror::Error;

pub const PROOF_OBLIGATIONS_PATH: &str = "contracts/proof_obligations.yaml";

pub const REQUIRED_PRODUCTION_OBLIGATIONS: &[&str] = &["verification/verus/receipt_schema.rs"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryAuthority {
    Production,
    SmokeFixture,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegistryEntry {
    pub path: String,
    pub authority: RegistryAuthority,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registry {
    pub version: u32,
    pub targets: Vec<RegistryEntry>,
}

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("registry is missing a `version:` line")]
    MissingVersion,
    #[error("registry has an invalid version on line {line}: `{value}`")]
    InvalidVersion { line: u32, value: String },
    #[error("registry is missing a `targets:` section")]
    MissingTargets,
    #[error("registry has an unexpected line {line}")]
    UnexpectedLine { line: u32 },
    #[error("registry has an empty path on line {line}")]
    EmptyPath { line: u32 },
    #[error("registry registers `{path}` twice")]
    DuplicatePath { path: String },
    #[error("registry has an invalid authority on line {line}: `{value}`")]
    InvalidAuthority { line: u32, value: String },
    #[error("registry has a misplaced authority on line {line}")]
    MisplacedAuthority { line: u32 },
}

pub fn parse_registry(content: &str) -> Result<Registry, RegistryError> {
    let state = content
        .lines()
        .zip(1_u32..)
        .try_fold(ParseState::new(), |state, (line, number)| parse_step(state, line, number))?;
    state.finish()
}

struct RawEntry {
    path: String,
    authority: Option<RegistryAuthority>,
}

struct ParseState {
    version: Option<u32>,
    seen_targets: bool,
    entries: Vec<RawEntry>,
}

impl ParseState {
    const fn new() -> Self {
        Self { version: None, seen_targets: false, entries: Vec::new() }
    }

    fn finish(self) -> Result<Registry, RegistryError> {
        let version = self.version.ok_or(RegistryError::MissingVersion)?;
        require_targets_section(self.seen_targets)?;
        let targets =
            self.entries.into_iter().map(RawEntry::finalize).collect::<Vec<RegistryEntry>>();
        Ok(Registry { version, targets })
    }
}

const fn require_targets_section(seen: bool) -> Result<(), RegistryError> {
    if seen { Ok(()) } else { Err(RegistryError::MissingTargets) }
}

impl RawEntry {
    fn finalize(self) -> RegistryEntry {
        let authority = self.authority.map_or(RegistryAuthority::Production, |authority| authority);
        RegistryEntry { path: self.path, authority }
    }
}

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

fn parse_targets_header(state: ParseState, number: u32) -> Result<ParseState, RegistryError> {
    if state.seen_targets {
        return Err(RegistryError::UnexpectedLine { line: number });
    }
    Ok(ParseState { version: state.version, seen_targets: true, entries: state.entries })
}

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
