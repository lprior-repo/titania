//! Reads v1 lane-artifact JSON files in `GateScope` order.
//!
//! Each artifact lives at `<target_root>/.titania/out/<scope_dir>/<lane>.json`.
//! The reader enumerates every regular `*.json` file in the scope directory,
//! deserialises each one, and returns a vector of `(Lane, LaneOutcome)` tuples
//! in the canonical [`GateScope::lanes`] order.
//!
//! Missing artifact files are not fatal to aggregation: the missing lane is
//! returned as `LaneOutcome::Failed { failure: LaneFailure::Infra { reason:
//! "output file missing" } }` so the final report records a gate failure instead
//! of silently skipping or aborting.
//!
//! # Errors
//!
//! - Malformed JSON or lane-name mismatch → [`ReaderError::InputError`].
//! - Non-`NotFound` filesystem errors → [`ReaderError::InputError`].
//! - Unexpected artifact file (stem does not name a known lane, or the named
//!   lane is not part of the requested scope) → [`ReaderError::InputError`].
//! - Two or more artifact files that resolve to the same lane identity →
//!   [`ReaderError::InputError`].

use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

/// Callback invoked for each regular JSON file in the artifact directory.
type ArtifactVisitor<'a> = dyn FnMut(&Path) -> Result<(), ReaderError> + 'a;

use thiserror::Error;
use titania_core::{GateScope, Lane, LaneArtifact, LaneFailure, LaneOutcome};

/// Errors returned by the lane-artifact reader.
#[derive(Debug, Error)]
pub enum ReaderError {
    /// Infrastructure failure — expected artifact file could not be read.
    #[error("infra failure for tool {tool}: {reason}")]
    InfraFailure {
        /// Lane tool that could not produce its artifact.
        tool: String,
        /// Stable, machine-readable reason.
        reason: String,
    },
    /// Input error — malformed JSON or lane-name mismatch.
    #[error("input error for lane {lane}: {cause}")]
    InputError {
        /// Lane that the artifact was expected for.
        lane: Lane,
        /// Human-readable cause.
        cause: String,
    },
    /// The requested scope is not supported by this v1 reader.
    #[error("unsupported gate scope {scope}")]
    UnsupportedScope {
        /// Debug representation of the unsupported scope.
        scope: String,
    },
}

/// Result of reading lane artifacts for a [`GateScope`].
pub type ReaderResult = Result<Vec<(Lane, LaneOutcome)>, ReaderError>;

/// Read all lane-artifact JSON files for the given scope at `target_root`.
///
/// The returned `Vec` is ordered exactly as [`GateScope::lanes`] prescribes.
/// Every regular `*.json` file under `<target_root>/.titania/out/<scope>/` is
/// inspected, so a stray or duplicated file is reported as an input error
/// instead of being silently ignored.
///
/// # Errors
///
/// Returns one [`LaneOutcome::Failed { .. }`] per missing lane output, preserving
/// scope order. Returns [`ReaderError::InputError`] when an existing artifact
/// cannot be read, parsed, or matched to its lane, when the directory contains
/// a file whose stem does not name a scoped lane, or when the directory
/// contains more than one artifact for the same lane. Returns
/// [`ReaderError::UnsupportedScope`] for future gate-scope variants unknown to
/// this v1 reader.
pub fn read_lane_artifacts(target_root: &Path, scope: GateScope) -> ReaderResult {
    let scope_dir = scope_dir(scope)?;
    let out_dir = artifact_dir(target_root, scope_dir);
    let expected: &[Lane] = scope.lanes();
    let scoped_lanes = scoped_lane_set(scope);

    let mut by_lane: HashMap<Lane, String> = HashMap::new();
    enumerate_artifact_files(&out_dir, scope_dir, &mut |entry: &Path| {
        classify_and_record(entry, scope_dir, expected, &scoped_lanes, &mut by_lane)
    })?;

    let parse = |contents: String, lane: Lane| parse_artifact(&contents, lane);
    expected
        .iter()
        .map(|lane| {
            by_lane
                .remove(lane)
                .map_or_else(|| Ok((*lane, missing_lane_outcome(*lane))), |c| parse(c, *lane))
        })
        .collect()
}

/// Map an enumerated artifact entry to a lane and record its content.
///
/// # Errors
///
/// Returns [`ReaderError::InputError`] for unknown stems, lanes outside the
/// scope, duplicate lane identities, or filesystem read failures.
fn classify_and_record(
    entry: &Path,
    scope_dir: &'static str,
    expected: &[Lane],
    scoped_lanes: &HashSet<Lane>,
    by_lane: &mut HashMap<Lane, String>,
) -> Result<(), ReaderError> {
    let stem = entry_stem(entry, expected, scope_dir)?;
    let lane = stem_to_lane(stem).ok_or_else(|| ReaderError::InputError {
        lane: first_scoped_lane(expected),
        cause: format!(
            "unexpected artifact file {:?} in scope {scope_dir}: stem {stem:?} does not name a known lane",
            entry.file_name(),
        ),
    })?;
    if !scoped_lanes.contains(&lane) {
        return Err(ReaderError::InputError {
            lane,
            cause: format!(
                "unexpected artifact file {:?} for lane {lane} in scope {scope_dir} (not part of this gate)",
                entry.file_name(),
            ),
        });
    }
    let Some(contents) = read_entry(entry, lane)? else {
        return Ok(());
    };
    if by_lane.insert(lane, contents).is_some() {
        return Err(ReaderError::InputError {
            lane,
            cause: format!("duplicate artifact files for lane {lane} in scope {scope_dir}"),
        });
    }
    Ok(())
}
/// Extract the UTF-8 file stem from an enumerated entry, or return a typed error.
///
/// # Errors
///
/// Returns [`ReaderError::InputError`] when the entry has no file name or
/// the file name is not valid UTF-8.
fn entry_stem<'a>(
    entry: &'a Path,
    expected: &[Lane],
    scope_dir: &'static str,
) -> Result<&'a str, ReaderError> {
    let invalid = || ReaderError::InputError {
        lane: first_scoped_lane(expected),
        cause: format!(
            "unexpected artifact file {:?} in scope {scope_dir}: filename is not valid UTF-8",
            entry.file_name(),
        ),
    };
    entry.file_stem().and_then(|name| name.to_str()).ok_or_else(invalid)
}

/// Read the artifact file at `entry` and convert non-`NotFound` I/O errors.
///
/// # Errors
///
fn read_entry(entry: &Path, lane: Lane) -> Result<Option<String>, ReaderError> {
    match std::fs::read_to_string(entry) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(ReaderError::InputError {
            lane,
            cause: format!("IO error reading artifact: {error}"),
        }),
    }
}

/// Parse one artifact payload and reconcile its embedded lane identity.
///
/// # Errors
///
/// Returns [`ReaderError::InputError`] when the file cannot be decoded as JSON,
/// the deserialised [`LaneArtifact`] does not name `lane`, or the projected
/// outcome is malformed.
fn parse_artifact(contents: &str, lane: Lane) -> Result<(Lane, LaneOutcome), ReaderError> {
    let artifact: LaneArtifact = serde_json::from_str(contents).map_err(|err| {
        ReaderError::InputError { lane, cause: format!("malformed JSON for {lane}: {err}") }
    })?;

    let artifact_lane = artifact.lane();
    if artifact_lane != lane {
        return Err(ReaderError::InputError {
            lane,
            cause: format!("lane mismatch in artifact: expected {lane}, got {artifact_lane}"),
        });
    }

    let outcome: LaneOutcome =
        artifact.into_outcome().into_lane_outcome().map_err(|err| ReaderError::InputError {
            lane,
            cause: format!("failed to parse outcome for {lane}: {err}"),
        })?;

    Ok((artifact_lane, outcome))
}
/// Enumerate the regular `*.json` files under `out_dir` in deterministic order
/// and invoke `visit` on each entry.
///
/// A missing directory is treated as zero artifacts so the caller can record
/// each scoped lane as missing.
///
/// # Errors
/// Returns [`ReaderError::InputError`] for non-`NotFound` directory read errors,
/// any error encountered while enumerating a single directory entry, or any
/// error produced by `visit`.
fn enumerate_artifact_files(
    out_dir: &Path,
    scope_dir: &'static str,
    visit: &mut ArtifactVisitor<'_>,
) -> Result<(), ReaderError> {
    let entries = match std::fs::read_dir(out_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => {
            return Err(ReaderError::InputError {
                lane: Lane::Fmt,
                cause: format!(
                    "IO error reading artifact directory {} for scope {scope_dir}: {error}",
                    out_dir.display(),
                ),
            });
        }
    };
    let mut paths: Vec<PathBuf> = entries
        .map(|entry| {
            entry.map(|e| e.path()).map_err(|error| ReaderError::InputError {
                lane: Lane::Fmt,
                cause: format!(
                    "IO error enumerating artifact directory {} for scope {scope_dir}: {error}",
                    out_dir.display(),
                ),
            })
        })
        .filter_map(|entry| match entry {
            Ok(path)
                if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") =>
            {
                Some(Ok(path))
            }
            Ok(_) => None,
            Err(error) => Some(Err(error)),
        })
        .collect::<Result<Vec<_>, ReaderError>>()?;
    paths.sort();
    paths.iter().try_fold((), |(), path| visit(path.as_path()))?;
    Ok(())
}
/// Build the set of lanes that participate in `scope`.
fn scoped_lane_set(scope: GateScope) -> HashSet<Lane> {
    scope.lanes().iter().copied().collect()
}

/// Resolve a filename stem to the [`Lane`] it names, if any.
fn stem_to_lane(stem: &str) -> Option<Lane> {
    match stem {
        "fmt" => Some(Lane::Fmt),
        "compile" => Some(Lane::Compile),
        "clippy" => Some(Lane::Clippy),
        "ast-grep" => Some(Lane::AstGrep),
        "dylint" => Some(Lane::Dylint),
        "panic-scan" => Some(Lane::PanicScan),
        "policy-scan" => Some(Lane::PolicyScan),
        "test" => Some(Lane::Test),
        "deny" => Some(Lane::Deny),
        "build" => Some(Lane::Build),
        _ => None,
    }
}

/// Return the first lane in `expected`, or [`Lane::Fmt`] when none is given.
///
/// Used to anchor directory-level errors to a concrete lane for the
/// [`ReaderError::InputError`] payload.
const fn first_scoped_lane(expected: &[Lane]) -> Lane {
    match expected.first() {
        Some(lane) => *lane,
        None => Lane::Fmt,
    }
}

fn missing_lane_outcome(lane: Lane) -> LaneOutcome {
    LaneOutcome::Failed {
        failure: LaneFailure::Infra {
            tool: lane.name().to_owned(),
            reason: "output file missing".to_owned(),
        },
    }
}

/// Return the output directory name for a gate scope.
///
/// # Errors
///
/// Returns [`ReaderError::UnsupportedScope`] for future gate-scope variants
/// unknown to this v1 reader.
fn scope_dir(scope: GateScope) -> Result<&'static str, ReaderError> {
    match scope {
        GateScope::Edit => Ok("edit"),
        GateScope::Prepush => Ok("prepush"),
        GateScope::Release => Ok("release"),
        _ => Err(ReaderError::UnsupportedScope { scope: format!("{scope:?}") }),
    }
}

/// Build the artifact output directory path for a scope.
fn artifact_dir(target_root: &Path, scope_dir: &str) -> PathBuf {
    target_root.join(".titania").join("out").join(scope_dir)
}

#[cfg(test)]
mod tests {
    //! Direct unit tests for internal helpers that are unreachable through the
    //! public `read_lane_artifacts` API on a normal case-sensitive filesystem.
    //!
    //! The on-disk stem-to-lane mapping is one-to-one and the directory
    //! enumerator visits each file once, so two regular files cannot resolve
    //! to the same lane identity via `read_lane_artifacts` alone. The
    //! duplicate-detection branch in [`classify_and_record`] is exercised
    //! here by pre-populating the destination map.
    use std::{
        collections::{HashMap, HashSet},
        fs,
        path::Path,
    };

    use tempfile::TempDir;
    use titania_core::{GateScope, Lane};

    use super::{ReaderError, classify_and_record, scoped_lane_set};

    #[test]
    fn classify_and_record_rejects_duplicate_lane_identity() {
        let tmp = TempDir::new().unwrap();
        let entry_path = tmp.path().join("fmt.json");
        fs::write(&entry_path, "{\"lane\":\"Fmt\",\"outcome\":{\"Skipped\":\"NotApplicable\"}}")
            .unwrap();
        let entry: &Path = entry_path.as_path();
        let scope_dir = "edit";
        let expected: &[Lane] = GateScope::Edit.lanes();
        let scoped_lanes: HashSet<Lane> = scoped_lane_set(GateScope::Edit);
        let mut by_lane: HashMap<Lane, String> = HashMap::new();
        let _existing = by_lane.insert(Lane::Fmt, String::from("already present"));

        let result = classify_and_record(entry, scope_dir, expected, &scoped_lanes, &mut by_lane);
        match result {
            Err(ReaderError::InputError { lane, cause }) => {
                assert_eq!(lane, Lane::Fmt);
                assert!(cause.contains("duplicate"), "cause was: {cause}");
            }
            other => panic!("expected duplicate InputError, got {other:?}"),
        }
    }

    #[test]
    fn classify_and_record_surfaces_non_not_found_read_errors() {
        let tmp = TempDir::new().unwrap();
        // A directory whose stem still resolves to a scoped lane forces
        // `classify_and_record` past the stem lookup and into `read_entry`,
        // where `read_to_string` reports a non-`NotFound` IO error.
        let entry_path = tmp.path().join("fmt");
        fs::create_dir(&entry_path).unwrap();
        let entry: &Path = entry_path.as_path();
        let scope_dir = "edit";
        let expected: &[Lane] = GateScope::Edit.lanes();
        let scoped_lanes: HashSet<Lane> = scoped_lane_set(GateScope::Edit);
        let mut by_lane: HashMap<Lane, String> = HashMap::new();

        let result = classify_and_record(entry, scope_dir, expected, &scoped_lanes, &mut by_lane);
        match result {
            Err(ReaderError::InputError { lane, cause }) => {
                assert_eq!(lane, Lane::Fmt);
                assert!(cause.contains("IO error reading artifact"), "cause was: {cause}");
            }
            other => panic!("expected IO InputError, got {other:?}"),
        }
    }
}
