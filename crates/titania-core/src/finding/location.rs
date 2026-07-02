use serde::{Deserialize, Serialize};

use crate::{error::LocationError, workspace_path::WorkspacePath};

/// Location where a finding was observed.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
pub enum Location {
    /// A byte-offset span within a single source file.
    Span { file: WorkspacePath, line_start: u32, col_start: u32, line_end: u32, col_end: u32 },
    /// A dependency crate and its version.
    Dependency { crate_name: String, version: String },
    /// A manifest file (Cargo.toml, policy.toml, etc.).
    Manifest { file: WorkspacePath },
    /// A workspace-level observation (no file or crate context).
    Workspace,
    /// A tool or its version.
    Tool { name: String, version: String },
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
        if line_start < 1 {
            return Err(LocationError::LineStartBeforeOne);
        }
        if line_end < line_start {
            return Err(LocationError::EndBeforeStart { line_start, line_end });
        }
        if col_end < col_start {
            return Err(LocationError::ColEndBeforeStart { col_start, col_end });
        }
        Ok(Self::Span { file, line_start, col_start, line_end, col_end })
    }

    /// Construct a [`Location::Dependency`].
    #[must_use]
    pub const fn dependency(crate_name: String, version: String) -> Self {
        Self::Dependency { crate_name, version }
    }

    /// Construct a [`Location::Manifest`].
    #[must_use]
    pub const fn manifest(file: WorkspacePath) -> Self {
        Self::Manifest { file }
    }

    /// Construct a [`Location::Workspace`].
    #[must_use]
    pub const fn workspace() -> Self {
        Self::Workspace
    }

    /// Construct a [`Location::Tool`].
    #[must_use]
    pub const fn tool(name: String, version: String) -> Self {
        Self::Tool { name, version }
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

    /// Return the span file or a typed error if this is not a span location.
    ///
    /// # Errors
    /// Returns [`LocationError::NotSpan`] for non-span variants.
    pub const fn as_file(&self) -> Result<&WorkspacePath, LocationError> {
        match self {
            Self::Span { file, .. } => Ok(file),
            _ => Err(LocationError::NotSpan),
        }
    }

    /// Return the span start line, or `0` for non-span locations.
    #[must_use]
    pub const fn line_start(&self) -> u32 {
        match self {
            Self::Span { line_start, .. } => *line_start,
            _ => 0,
        }
    }

    /// Return the span start column, or `0` for non-span locations.
    #[must_use]
    pub const fn col_start(&self) -> u32 {
        match self {
            Self::Span { col_start, .. } => *col_start,
            _ => 0,
        }
    }

    /// Return the span end line, or `0` for non-span locations.
    #[must_use]
    pub const fn line_end(&self) -> u32 {
        match self {
            Self::Span { line_end, .. } => *line_end,
            _ => 0,
        }
    }

    /// Return the span end column, or `0` for non-span locations.
    #[must_use]
    pub const fn col_end(&self) -> u32 {
        match self {
            Self::Span { col_end, .. } => *col_end,
            _ => 0,
        }
    }
}

#[derive(Deserialize)]
#[serde(tag = "variant", rename_all = "snake_case")]
enum LocationWire {
    Span { file: WorkspacePath, line_start: u32, col_start: u32, line_end: u32, col_end: u32 },
    Dependency { crate_name: String, version: String },
    Manifest { file: WorkspacePath },
    Workspace,
    Tool { name: String, version: String },
}

impl TryFrom<LocationWire> for Location {
    type Error = LocationError;

    fn try_from(wire: LocationWire) -> Result<Self, Self::Error> {
        match wire {
            LocationWire::Span { file, line_start, col_start, line_end, col_end } => {
                Self::span(file, line_start, col_start, line_end, col_end)
            }
            LocationWire::Dependency { crate_name, version } => {
                Ok(Self::Dependency { crate_name, version })
            }
            LocationWire::Manifest { file } => Ok(Self::Manifest { file }),
            LocationWire::Workspace => Ok(Self::Workspace),
            LocationWire::Tool { name, version } => Ok(Self::Tool { name, version }),
        }
    }
}

impl<'de> Deserialize<'de> for Location {
    fn deserialize<D: serde::Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        LocationWire::deserialize(de)?.try_into().map_err(serde::de::Error::custom)
    }
}
