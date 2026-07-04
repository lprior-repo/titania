//! Reads v1 lane-artifact JSON files in `GateScope` order.
//!
//! Each artifact lives at `<target_root>/.titania/out/<scope_dir>/<lane>.json`.
//! The reader walks the lanes for the requested scope, deserialises each file,
//! and returns a vector of `(Lane, LaneOutcome)` tuples in scope order.
//!
//! # Errors
//!
//! - Missing artifact file → [`ReaderError::InfraFailure`] with reason
//!   `"output file missing"`.
//! - Malformed JSON or lane-name mismatch → [`ReaderError::InputError`].

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;
use thiserror::Error;
use titania_core::{GateScope, Lane, LaneOutcome};

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
/// Returns [`ReaderError::InfraFailure`] when an expected lane artifact is
/// missing, [`ReaderError::InputError`] when an artifact cannot be parsed or its
/// embedded lane does not match its filename, and
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
/// Returns [`ReaderError::InfraFailure`] when the lane output file is missing,
/// or [`ReaderError::InputError`] when the file cannot be read, decoded as
/// JSON, deserialized as a [`LaneOutcome`], or its embedded lane name does not
/// match the expected `lane`.
fn read_one(out_dir: &Path, lane: Lane) -> Result<(Lane, LaneOutcome), ReaderError> {
    let file_path = out_dir.join(lane_stem(lane)).with_extension("json");
    let contents = read_artifact_file(&file_path, lane)?;
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
/// Returns [`ReaderError::InfraFailure`] for missing files and
/// [`ReaderError::InputError`] for other filesystem errors.
fn read_artifact_file(file_path: &Path, lane: Lane) -> Result<String, ReaderError> {
    std::fs::read_to_string(file_path).map_err(|err| read_file_error(lane, &err))
}

fn read_file_error(lane: Lane, err: &std::io::Error) -> ReaderError {
    match err.kind() {
        std::io::ErrorKind::NotFound => ReaderError::InfraFailure {
            tool: lane.name().to_owned(),
            reason: "output file missing".to_owned(),
        },
        _ => ReaderError::InputError { lane, cause: format!("IO error reading artifact: {err}") },
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
    fn read_missing_dylint_artifact_returns_infra_failure() {
        let tmp = tempfile::tempdir().unwrap();
        let edit_dir = tmp.path().join(".titania").join("out").join("edit");
        fs::create_dir_all(&edit_dir).unwrap();

        // Write all lanes before Dylint (Fmt, Compile, Clippy, AstGrep).
        // Dylint is intentionally missing so we can verify InfraFailure.
        for lane in GateScope::Edit.lanes() {
            if *lane != Lane::Dylint && *lane != Lane::PanicScan && *lane != Lane::PolicyScan {
                let path = edit_dir.join(format!("{}.json", lane_stem(*lane)));
                fs::write(&path, clean_artifact_json(*lane)).unwrap();
            }
        }

        let result = read_lane_artifacts(tmp.path(), GateScope::Edit);
        // Fmt-Compile-Clippy-AstGrep succeed, Dylint InfraFailure
        match result {
            Err(ReaderError::InfraFailure { tool, reason }) => {
                assert_eq!(tool, "Dylint");
                assert_eq!(reason, "output file missing");
            }
            other => panic!("expected InfraFailure for Dylint, got {other:?}"),
        }
    }

    #[test]
    fn read_dylint_missing_returns_infra_failure_reason() {
        let tmp = tempfile::tempdir().unwrap();
        // No artifact directory at all — all lanes missing
        let result = read_lane_artifacts(tmp.path(), GateScope::Edit);
        match result {
            Err(ReaderError::InfraFailure { tool, reason }) => {
                assert_eq!(tool, "Fmt");
                assert_eq!(reason, "output file missing");
            }
            other => panic!("expected InfraFailure, got {other:?}"),
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
