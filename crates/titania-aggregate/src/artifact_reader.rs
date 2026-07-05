//! Reads v1 lane-artifact JSON files in `GateScope` order.
//!
//! Each artifact lives at `<target_root>/.titania/out/<scope_dir>/<lane>.json`.
//! The reader walks the lanes for the requested scope, deserialises each file,
//! and returns a vector of `(Lane, LaneOutcome)` tuples in scope order.
//!
//! Missing artifact files are not fatal to aggregation: the missing lane is
//! returned as `LaneOutcome::Failed(LaneFailure::Infra { reason:
//! "output file missing" })` so the final report records a gate failure instead
//! of silently skipping or aborting.
//!
//! # Errors
//!
//! - Malformed JSON or lane-name mismatch → [`ReaderError::InputError`].
//! - Non-`NotFound` filesystem errors → [`ReaderError::InputError`].

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use titania_core::{GateScope, Lane, LaneFailure, LaneOutcome};

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

/// One deserialised lane-artifact record (flexible outcome shape).
#[derive(Debug, Deserialize)]
struct LaneArtifact {
    lane: Lane,
    outcome: Value,
}

/// Result of reading lane artifacts for a [`GateScope`].
pub type ReaderResult = Result<Vec<(Lane, LaneOutcome)>, ReaderError>;

/// Read all lane-artifact JSON files for the given scope at `target_root`.
///
/// The returned `Vec` is ordered exactly as [`GateScope::lanes`] prescribes.
///
/// # Errors
///
/// Returns one [`LaneOutcome::Failed`] per missing lane output, preserving scope
/// order. Returns [`ReaderError::InputError`] when an existing artifact cannot be
/// read, parsed, or matched to its lane, and [`ReaderError::UnsupportedScope`]
/// [`ReaderError::UnsupportedScope`] for future gate-scope variants unknown to
/// this v1 reader.
pub fn read_lane_artifacts(target_root: &Path, scope: GateScope) -> ReaderResult {
    let scope_dir = scope_dir(scope)?;
    let out_dir = artifact_dir(target_root, scope_dir);

    scope.lanes().iter().map(|lane| read_one(&out_dir, *lane)).collect()
}

/// Read one expected lane artifact from `out_dir`.
///
/// # Errors
///
/// Returns [`LaneOutcome::Failed`] when the lane output file is missing, or
/// [`ReaderError::InputError`] when the file cannot be read, decoded as JSON,
/// deserialized as a [`LaneOutcome`], or its embedded lane name does not match
/// match the expected `lane`.
fn read_one(out_dir: &Path, lane: Lane) -> Result<(Lane, LaneOutcome), ReaderError> {
    let file_path = out_dir.join(lane_stem(lane)).with_extension("json");
    let Some(contents) = read_artifact_file(&file_path, lane)? else {
        return Ok((lane, missing_lane_outcome(lane)));
    };
    let artifact: LaneArtifact = serde_json::from_str(&contents).map_err(|err| {
        ReaderError::InputError { lane, cause: format!("malformed JSON for {lane}: {err}") }
    })?;

    if artifact.lane != lane {
        return Err(ReaderError::InputError {
            lane,
            cause: format!("lane mismatch in artifact: expected {lane}, got {}", artifact.lane),
        });
    }

    let outcome: LaneOutcome =
        serde_json::from_value(artifact.outcome).map_err(|err| ReaderError::InputError {
            lane,
            cause: format!("failed to parse outcome for {lane}: {err}"),
        })?;

    Ok((artifact.lane, outcome))
}

/// Read a lane artifact file.
///
/// # Errors
///
/// Returns `Ok(None)` for missing files and [`ReaderError::InputError`] for
/// other filesystem errors.
fn read_artifact_file(file_path: &Path, lane: Lane) -> Result<Option<String>, ReaderError> {
    match std::fs::read_to_string(file_path) {
        Ok(contents) => Ok(Some(contents)),
        Err(error) => read_file_error(lane, &error),
    }
}

/// Classify filesystem read errors for a lane artifact.
///
/// # Errors
///
/// Returns [`ReaderError::InputError`] for non-missing filesystem errors.
fn read_file_error(lane: Lane, err: &std::io::Error) -> Result<Option<String>, ReaderError> {
    match err.kind() {
        std::io::ErrorKind::NotFound => Ok(None),
        _ => Err(ReaderError::InputError {
            lane,
            cause: format!("IO error reading artifact: {err}"),
        }),
    }
}

fn missing_lane_outcome(lane: Lane) -> LaneOutcome {
    LaneOutcome::Failed(LaneFailure::Infra {
        tool: lane.name().to_owned(),
        reason: "output file missing".to_owned(),
    })
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

/// Return the filename stem for a lane (without the `.json` extension).
const fn lane_stem(lane: Lane) -> &'static str {
    match lane {
        Lane::Fmt => "fmt",
        Lane::Compile => "compile",
        Lane::Clippy => "clippy",
        Lane::AstGrep => "ast-grep",
        Lane::Dylint => "dylint",
        Lane::PanicScan => "panic-scan",
        Lane::PolicyScan => "policy-scan",
        Lane::Test => "test",
        Lane::Deny => "deny",
        Lane::Build => "build",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Build minimal valid clean-artifact JSON for a lane.
    fn clean_artifact_json(lane: Lane) -> String {
        format!(
            r#"{{"lane":"{}","outcome":{{"variant":"clean","evidence":{{"command":{{"executable":"cargo","argv":["cargo","check"]}},"tool_version":"1.0","exit_status":{{"exited":{{"code":0}}}},"parsed_result_digest":"0000000000000000000000000000000000000000000000000000000000000000"}}}}}}"#,
            lane.name()
        )
    }

    #[test]
    fn read_missing_dylint_artifact_returns_failed_lane_outcome() {
        let tmp = tempfile::tempdir().unwrap();
        let edit_dir = tmp.path().join(".titania").join("out").join("edit");
        fs::create_dir_all(&edit_dir).unwrap();

        for lane in GateScope::Edit.lanes() {
            if *lane != Lane::Dylint {
                let path = edit_dir.join(format!("{}.json", lane_stem(*lane)));
                fs::write(&path, clean_artifact_json(*lane)).unwrap();
            }
        }

        let outcomes = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
        assert_eq!(outcomes.len(), GateScope::Edit.lanes().len());
        assert_missing_lane(&outcomes, Lane::Dylint);
    }

    #[test]
    fn read_all_missing_returns_failed_lane_outcomes_in_scope_order() {
        let tmp = tempfile::tempdir().unwrap();
        let outcomes = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
        assert_eq!(outcomes.len(), GateScope::Edit.lanes().len());
        assert_missing_lane(&outcomes, Lane::Fmt);
        assert_missing_lane(&outcomes, Lane::Dylint);
        assert_missing_lane(&outcomes, Lane::PolicyScan);
    }

    fn assert_missing_lane(outcomes: &[(Lane, LaneOutcome)], lane: Lane) {
        let (_, outcome) = outcomes.iter().find(|(found, _)| *found == lane).unwrap();
        match outcome {
            LaneOutcome::Failed(LaneFailure::Infra { tool, reason }) => {
                assert_eq!(tool, lane.name());
                assert_eq!(reason, "output file missing");
            }
            other => panic!("expected missing-lane Infra failure for {lane}, got {other:?}"),
        }
    }

    #[test]
    fn malformed_json_returns_input_error() {
        let tmp = tempfile::tempdir().unwrap();
        let edit_dir = tmp.path().join(".titania").join("out").join("edit");
        fs::create_dir_all(&edit_dir).unwrap();

        let path = edit_dir.join("fmt.json");
        fs::write(&path, "NOT VALID JSON {{{").unwrap();

        let result = read_lane_artifacts(tmp.path(), GateScope::Edit);
        match result {
            Err(ReaderError::InputError { lane, cause }) => {
                assert_eq!(lane, Lane::Fmt);
                assert!(cause.contains("malformed JSON"));
            }
            other => panic!("expected InputError, got {other:?}"),
        }
    }

    #[test]
    fn lane_mismatch_returns_input_error() {
        let tmp = tempfile::tempdir().unwrap();
        let edit_dir = tmp.path().join(".titania").join("out").join("edit");
        fs::create_dir_all(&edit_dir).unwrap();

        // Write a file claiming to be for Compile lane but placed at fmt.json
        let path = edit_dir.join("fmt.json");
        fs::write(&path, r#"{"lane":"Compile","outcome":{"variant":"clean","evidence":null}}"#)
            .unwrap();

        let result = read_lane_artifacts(tmp.path(), GateScope::Edit);
        match result {
            Err(ReaderError::InputError { lane, cause }) => {
                assert_eq!(lane, Lane::Fmt);
                assert!(cause.contains("lane mismatch"));
            }
            other => panic!("expected InputError, got {other:?}"),
        }
    }

    #[test]
    fn scope_order_preserved() {
        let tmp = tempfile::tempdir().unwrap();
        let edit_dir = tmp.path().join(".titania").join("out").join("edit");
        fs::create_dir_all(&edit_dir).unwrap();

        // Write all Edit-lane artifacts with valid LaneEvidence
        for lane in GateScope::Edit.lanes() {
            let path = edit_dir.join(format!("{}.json", lane_stem(*lane)));
            fs::write(&path, clean_artifact_json(*lane)).unwrap();
        }

        let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
        assert_eq!(records.len(), 7); // 7 Edit lanes

        // Verify order matches GateScope::lanes
        for (i, &expected_lane) in GateScope::Edit.lanes().iter().enumerate() {
            assert_eq!(records[i].0, expected_lane, "lane at index {i} mismatch");
        }
    }

    #[test]
    fn scope_dir_mapping() {
        assert_eq!(scope_dir(GateScope::Edit).unwrap(), "edit");
        assert_eq!(scope_dir(GateScope::Prepush).unwrap(), "prepush");
        assert_eq!(scope_dir(GateScope::Release).unwrap(), "release");
    }
}
