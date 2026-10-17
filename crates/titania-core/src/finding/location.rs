use serde::{Deserialize, Serialize};

use crate::{error::LocationError, workspace_path::WorkspacePath};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
enum LocationInner {
    Span { file: WorkspacePath, line_start: u32, col_start: u32, line_end: u32, col_end: u32 },
    Dependency { crate_name: String, version: String },
    Manifest { file: WorkspacePath },
    Workspace,
    Tool { name: String, version: String },
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(transparent)]
pub struct Location(LocationInner);

impl Location {
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

    #[must_use]
    pub const fn dependency(crate_name: String, version: String) -> Self {
        Self(LocationInner::Dependency { crate_name, version })
    }

    #[must_use]
    pub const fn manifest(file: WorkspacePath) -> Self {
        Self(LocationInner::Manifest { file })
    }

    #[must_use]
    pub const fn workspace() -> Self {
        Self(LocationInner::Workspace)
    }

    #[must_use]
    pub const fn tool(name: String, version: String) -> Self {
        Self(LocationInner::Tool { name, version })
    }

    #[must_use]
    pub const fn is_span(&self) -> bool {
        self.0.is_span()
    }

    #[must_use]
    pub const fn span_file(&self) -> Option<&WorkspacePath> {
        self.0.span_file()
    }
}

#[derive(Deserialize)]
enum LocationReadWire {
    Span { file: WorkspacePath, line_start: u32, col_start: u32, line_end: u32, col_end: u32 },
    Dependency { crate_name: String, version: String },
    Manifest { file: WorkspacePath },
    Workspace,
    Tool { name: String, version: String },
}

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

const fn dep(crate_name: String, version: String) -> Location {
    Location(LocationInner::Dependency { crate_name, version })
}

const fn manifest(file: WorkspacePath) -> Location {
    Location(LocationInner::Manifest { file })
}

const fn workspace() -> Location {
    Location(LocationInner::Workspace)
}

const fn tool(name: String, version: String) -> Location {
    Location(LocationInner::Tool { name, version })
}

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
    if line_end == line_start && col_end < col_start {
        return Err(LocationError::ColEndBeforeStart { col_start, col_end });
    }
    Ok(())
}
