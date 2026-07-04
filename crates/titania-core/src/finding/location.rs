//! Location domain type and serde wire adapters.
//!
//! `Location` derives `Serialize` for production wire output. Deserialization
//! uses a private `LocationReadWire` intermediate and smart-constructors so that
//! validation (span bounds) runs on every deserialize path.

use serde::{Deserialize, Serialize};

use crate::{error::LocationError, workspace_path::WorkspacePath};

/// Location where a finding was observed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum Location {
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
        Ok(Self::Span { file, line_start, col_start, line_end, col_end })
    }

    /// Whether this location is a file span.
    #[must_use]
    pub const fn is_span(&self) -> bool {
        matches!(self, Self::Span { .. })
    }

    /// If this location is a span, return the workspace path.
    #[must_use]
    pub const fn span_file(&self) -> Option<&WorkspacePath> {
        match self {
            Self::Span { file, .. } => Some(file),
            _ => None,
        }
    }
}

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

#[derive(Deserialize)]
#[serde(tag = "variant", rename_all = "snake_case", deny_unknown_fields)]
enum LocationReadWire {
    Span { file: WorkspacePath, line_start: u32, col_start: u32, line_end: u32, col_end: u32 },
    Dependency { crate_name: String, version: String },
    Manifest { file: WorkspacePath },
    Workspace,
    Tool { name: String, version: String },
}

/// Convert deserialized location wire data into a validated domain location.
///
/// # Errors
/// Returns [`LocationError`] when span bounds are invalid.
fn location_from_wire(wire: LocationReadWire) -> Result<Location, LocationError> {
    match wire {
        LocationReadWire::Span { file, line_start, col_start, line_end, col_end } => {
            validate_span_bounds(line_start, col_start, line_end, col_end)
                .map(|()| Location::Span { file, line_start, col_start, line_end, col_end })
        }
        LocationReadWire::Dependency { crate_name, version } => {
            Ok(Location::Dependency { crate_name, version })
        }
        LocationReadWire::Manifest { file } => Ok(Location::Manifest { file }),
        LocationReadWire::Workspace => Ok(Location::Workspace),
        LocationReadWire::Tool { name, version } => Ok(Location::Tool { name, version }),
    }
}

impl TryFrom<LocationReadWire> for Location {
    type Error = LocationError;

    fn try_from(wire: LocationReadWire) -> Result<Self, Self::Error> {
        location_from_wire(wire)
    }
}

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = LocationReadWire::deserialize(deserializer)?;
        location_from_wire(wire).map_err(serde::de::Error::custom)
    }
}
