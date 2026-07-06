//! Location domain type and serde wire adapters.
//!
//! `Location` is a public newtype wrapper over a private `LocationInner` enum.
//! All construction goes through validated constructors (`Location::span`, etc.).
//! Direct variant construction is impossible because `LocationInner` is private.
//!
//! Serde deserialization uses a private `LocationReadWire` intermediate so that
//! validation runs on every deserialize path.

use serde::{Deserialize, Serialize};

use crate::{error::LocationError, workspace_path::WorkspacePath};

// ── Private inner enum ──────────────────────────────────────────────────────

/// Private inner type for [`Location`].
///
/// Sealed so external crates cannot construct variants directly.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
enum LocationInner {
    /// A byte-offset span within a single source file.
    Span {
        /// Workspace-relative source file path containing the finding.
        file: WorkspacePath,
        /// One-based first line of the finding span.
        line_start: u32,
        /// Zero-based first column of the finding span.
        col_start: u32,
        /// One-based last line of the finding span.
        line_end: u32,
        /// Zero-based last column of the finding span.
        col_end: u32,
    },
    /// A dependency crate and its version.
    Dependency {
        /// Dependency package name.
        crate_name: String,
        /// Dependency version observed by the lane.
        version: String,
    },
    /// A manifest file (Cargo.toml, policy.toml, etc.).
    Manifest {
        /// Workspace-relative manifest path.
        file: WorkspacePath,
    },
    /// A workspace-level observation (no file or crate context).
    Workspace,
    /// A tool or its version.
    Tool {
        /// Tool executable or logical tool name.
        name: String,
        /// Tool version string reported by the lane.
        version: String,
    },
}

impl LocationInner {
    const fn is_span(&self) -> bool {
        matches!(self, Self::Span { .. })
    }

    const fn span_file(&self) -> Option<&WorkspacePath> {
        match self {
            Self::Span { file, .. } => Some(file),
            _ => None,
        }
    }
}

// ── Public newtype ──────────────────────────────────────────────────────────

/// Location where a finding was observed.
///
/// This is a newtype wrapper over a private inner enum. All construction
/// goes through smart constructors that enforce invariants.
///
/// # Serialization
///
/// `Location` derives `Serialize` for production wire output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Location(LocationInner);

impl Location {
    /// Construct a [`Location::Span`].
    ///
    /// # Errors
    /// - [`LocationError::LineStartBeforeOne`] if `line_start < 1`.
    /// - [`LocationError::EndBeforeStart`] if `line_end < line_start`.
    /// - [`LocationError::ColEndBeforeStart`] if `col_end < col_start`.
    ///
    /// Line numbers are 1-based; column numbers are 0-based Unicode scalar
    /// values.
    pub fn span(
        file: WorkspacePath,
        line_start: u32,
        col_start: u32,
        line_end: u32,
        col_end: u32,
    ) -> Result<Self, LocationError> {
        validate_span_bounds(line_start, col_start, line_end, col_end)?;
        Ok(Self(LocationInner::Span { file, line_start, col_start, line_end, col_end }))
    }

    /// Construct a [`Location::Dependency`].
    #[must_use]
    pub const fn dependency(crate_name: String, version: String) -> Self {
        Self(LocationInner::Dependency { crate_name, version })
    }

    /// Construct a [`Location::Manifest`].
    #[must_use]
    pub const fn manifest(file: WorkspacePath) -> Self {
        Self(LocationInner::Manifest { file })
    }

    /// Construct a [`Location::Workspace`].
    #[must_use]
    pub const fn workspace() -> Self {
        Self(LocationInner::Workspace)
    }

    /// Construct a [`Location::Tool`].
    #[must_use]
    pub const fn tool(name: String, version: String) -> Self {
        Self(LocationInner::Tool { name, version })
    }

    /// Whether this location is a file span.
    #[must_use]
    pub const fn is_span(&self) -> bool {
        self.0.is_span()
    }

    /// If this location is a span, return the workspace path.
    #[must_use]
    pub const fn span_file(&self) -> Option<&WorkspacePath> {
        self.0.span_file()
    }
}

// ── Wire deserialization ────────────────────────────────────────────────────

/// Intermediate wire representation for [`Location`] deserialization.
///
/// Private — external crates cannot construct or inspect variants directly.
#[derive(Deserialize)]
#[serde(tag = "variant", rename_all = "snake_case", deny_unknown_fields)]
enum LocationReadWire {
    Span { file: WorkspacePath, line_start: u32, col_start: u32, line_end: u32, col_end: u32 },
    Dependency { crate_name: String, version: String },
    Manifest { file: WorkspacePath },
    Workspace,
    Tool { name: String, version: String },
}

/// Construct a [`Location::Span`].
///
/// # Errors
/// - [`LocationError::LineStartBeforeOne`] if `line_start < 1`.
/// - [`LocationError::EndBeforeStart`] if `line_end < line_start`.
/// - [`LocationError::ColEndBeforeStart`] if `col_end < col_start`.
fn construct_span(
    file: WorkspacePath,
    line_start: u32,
    col_start: u32,
    line_end: u32,
    col_end: u32,
) -> Result<Location, LocationError> {
    validate_span_bounds(line_start, col_start, line_end, col_end)?;
    Ok(Location(LocationInner::Span { file, line_start, col_start, line_end, col_end }))
}

/// Construct a [`Location::Dependency`].
const fn dep(crate_name: String, version: String) -> Location {
    Location(LocationInner::Dependency { crate_name, version })
}

/// Construct a [`Location::Manifest`].
const fn manifest(file: WorkspacePath) -> Location {
    Location(LocationInner::Manifest { file })
}

/// Construct a [`Location::Workspace`].
const fn workspace() -> Location {
    Location(LocationInner::Workspace)
}

/// Construct a [`Location::Tool`].
const fn tool(name: String, version: String) -> Location {
    Location(LocationInner::Tool { name, version })
}

/// Convert deserialized location wire data into a validated domain location.
///
/// # Errors
/// - [`LocationError::LineStartBeforeOne`], [`LocationError::EndBeforeStart`],
///   or [`LocationError::ColEndBeforeStart`] when span coordinates are invalid.
fn location_from_wire(wire: LocationReadWire) -> Result<Location, LocationError> {
    let location = match wire {
        LocationReadWire::Span { file, line_start, col_start, line_end, col_end } => {
            construct_span(file, line_start, col_start, line_end, col_end)?
        }
        LocationReadWire::Dependency { crate_name, version } => dep(crate_name, version),
        LocationReadWire::Manifest { file } => manifest(file),
        LocationReadWire::Workspace => workspace(),
        LocationReadWire::Tool { name, version } => tool(name, version),
    };
    Ok(location)
}

/// Deserialize a [`Location`] from wire format.
///
/// # Errors
/// Propagates serde deserialization errors.
fn deserialize_location<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Location, D::Error> {
    let wire = LocationReadWire::deserialize(deserializer)?;
    location_from_wire(wire).map_err(serde::de::Error::custom)
}

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserialize_location(deserializer)
    }
}

// ── Span validation ─────────────────────────────────────────────────────────

/// Validate span coordinate invariants.
///
/// # Errors
/// - [`LocationError::LineStartBeforeOne`] when `line_start < 1`.
/// - [`LocationError::EndBeforeStart`] when `line_end < line_start`.
/// - [`LocationError::ColEndBeforeStart`] when `col_end < col_start`.
const fn validate_span_bounds(
    line_start: u32,
    col_start: u32,
    line_end: u32,
    col_end: u32,
) -> Result<(), LocationError> {
    if line_start < 1 {
        return Err(LocationError::LineStartBeforeOne);
    }
    if line_end < line_start {
        return Err(LocationError::EndBeforeStart { line_start, line_end });
    }
    if col_end < col_start {
        return Err(LocationError::ColEndBeforeStart { col_start, col_end });
    }
    Ok(())
}
