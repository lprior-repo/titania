//! Integration / behaviour tests for the artifact reader.
//!
//! These tests exercise the public API surface only:
//! `read_lane_artifacts`, `ReaderError`, and `ReaderResult`.

use std::fs;

use tempfile::TempDir;
use titania_aggregate::{ReaderError, read_lane_artifacts};
use titania_core::{GateScope, Lane, LaneFailure, LaneOutcome};

/// Helper: build a minimal clean artifact JSON string for a given lane.
fn clean_artifact_json(lane: Lane) -> String {
    format!(
        r#"{{"lane":"{}","outcome":{{"Clean":{{"evidence":{{"command":{{"executable":"cargo","argv":["cargo","check"]}},"tool_version":"1.0","exit_status":{{"Exited":{{"code":0}}}},"parsed_result_digest":"0000000000000000000000000000000000000000000000000000000000000000"}}}}}}}}"#,
        lane.name()
    )
}

/// Helper: build a skipped-artifact JSON string.
fn skipped_artifact_json(lane: Lane) -> String {
    format!(r#"{{"lane":"{}","outcome":{{"Skipped":"NotSelectedByScope"}}}}"#, lane.name())
}

/// Helper: create the artifact directory structure for a scope.
fn setup_scope_dir(tmp: &TempDir, scope: GateScope) -> std::path::PathBuf {
    let scope_name = match scope {
        GateScope::Edit => "edit",
        GateScope::Prepush => "prepush",
        GateScope::Release => "release",
        _ => unreachable!("future GateScope variant"),
    };
    let dir = tmp.path().join(".titania").join("out").join(scope_name);
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn read_all_edit_lanes_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);

    for lane in GateScope::Edit.lanes() {
        fs::write(dir.join(format!("{}.json", lane_stem(*lane))), clean_artifact_json(*lane))
            .unwrap();
    }

    let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
    assert_eq!(records.len(), 7);

    for (i, &expected) in GateScope::Edit.lanes().iter().enumerate() {
        assert_eq!(records[i].0, expected);
        assert!(matches!(records[i].1, LaneOutcome::Clean { .. }));
    }
}

#[test]
fn read_all_prepush_lanes_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Prepush);

    for lane in GateScope::Prepush.lanes() {
        fs::write(dir.join(format!("{}.json", lane_stem(*lane))), clean_artifact_json(*lane))
            .unwrap();
    }

    let records = read_lane_artifacts(tmp.path(), GateScope::Prepush).unwrap();
    assert_eq!(records.len(), 9);

    for (i, &expected) in GateScope::Prepush.lanes().iter().enumerate() {
        assert_eq!(records[i].0, expected);
    }
}

#[test]
fn read_all_release_lanes_in_order() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Release);

    for lane in GateScope::Release.lanes() {
        fs::write(dir.join(format!("{}.json", lane_stem(*lane))), clean_artifact_json(*lane))
            .unwrap();
    }

    let records = read_lane_artifacts(tmp.path(), GateScope::Release).unwrap();
    assert_eq!(records.len(), 10);

    for (i, &expected) in GateScope::Release.lanes().iter().enumerate() {
        assert_eq!(records[i].0, expected);
    }
}

#[test]
fn missing_first_lane_returns_failed_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);

    fs::write(dir.join("compile.json"), clean_artifact_json(Lane::Compile)).unwrap();

    let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
    assert_missing_lane(&records, Lane::Fmt);
}

#[test]
fn missing_middle_lane_returns_failed_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);

    for lane in GateScope::Edit.lanes().iter().take(3) {
        fs::write(dir.join(format!("{}.json", lane_stem(*lane))), clean_artifact_json(*lane))
            .unwrap();
    }

    let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
    assert_missing_lane(&records, Lane::AstGrep);
}

#[test]
fn malformed_json_returns_input_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);

    fs::write(dir.join("fmt.json"), "this is not json {{{").unwrap();

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
    let dir = setup_scope_dir(&tmp, GateScope::Edit);

    // Place a Compile artifact at the Fmt file path
    fs::write(dir.join("fmt.json"), clean_artifact_json(Lane::Compile)).unwrap();

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
fn skipped_lane_parsed_correctly() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Prepush);

    for lane in GateScope::Prepush.lanes() {
        let json = if *lane == Lane::PolicyScan {
            skipped_artifact_json(*lane)
        } else {
            clean_artifact_json(*lane)
        };
        fs::write(dir.join(format!("{}.json", lane_stem(*lane))), json).unwrap();
    }

    let records = read_lane_artifacts(tmp.path(), GateScope::Prepush).unwrap();
    assert_eq!(records.len(), 9);

    let policy_record = records.iter().find(|(l, _)| *l == Lane::PolicyScan).unwrap();
    assert!(matches!(policy_record.1, LaneOutcome::Skipped { .. }));
}

#[test]
fn empty_target_root_returns_failed_outcomes() {
    let tmp = tempfile::tempdir().unwrap();

    let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
    assert_missing_lane(&records, Lane::Fmt);
    assert_missing_lane(&records, Lane::Dylint);
    assert_missing_lane(&records, Lane::PolicyScan);
}

#[test]
fn dylint_specific_missing_file_becomes_failed_outcome() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);

    for lane in GateScope::Edit.lanes() {
        if *lane != Lane::Dylint {
            fs::write(dir.join(format!("{}.json", lane_stem(*lane))), clean_artifact_json(*lane))
                .unwrap();
        }
    }

    let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
    assert_missing_lane(&records, Lane::Dylint);
}

#[test]
fn unexpected_unknown_stem_returns_input_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);

    // Drop a file whose stem does not name any known v1 lane.
    fs::write(dir.join("extra.json"), clean_artifact_json(Lane::Fmt)).unwrap();

    let result = read_lane_artifacts(tmp.path(), GateScope::Edit);
    match result {
        Err(ReaderError::InputError { lane, cause }) => {
            assert_eq!(lane, GateScope::Edit.lanes()[0]);
            assert!(cause.contains("unexpected artifact file"));
            assert!(cause.contains("\"extra.json\""));
            assert!(cause.contains("does not name a known lane"));
        }
        other => panic!("expected InputError, got {other:?}"),
    }
}

#[test]
fn unexpected_out_of_scope_lane_returns_input_error() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);

    // Write a clean Edit scope, then drop a `build.json` artifact at the same dir.
    for lane in GateScope::Edit.lanes() {
        fs::write(dir.join(format!("{}.json", lane_stem(*lane))), clean_artifact_json(*lane))
            .unwrap();
    }
    fs::write(dir.join("build.json"), clean_artifact_json(Lane::Build)).unwrap();

    let result = read_lane_artifacts(tmp.path(), GateScope::Edit);
    match result {
        Err(ReaderError::InputError { lane, cause }) => {
            assert_eq!(lane, Lane::Build);
            assert!(cause.contains("unexpected artifact file"));
            assert!(cause.contains("not part of this gate"));
        }
        other => panic!("expected InputError, got {other:?}"),
    }
}

#[test]
fn non_json_files_in_scope_dir_are_ignored() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);

    // Drop a stray non-JSON file alongside the expected artifacts.
    fs::write(dir.join("README.txt"), "this lane is fake").unwrap();
    fs::write(dir.join("notes.md"), "# scratch").unwrap();

    for lane in GateScope::Edit.lanes() {
        fs::write(dir.join(format!("{}.json", lane_stem(*lane))), clean_artifact_json(*lane))
            .unwrap();
    }

    let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
    assert_eq!(records.len(), GateScope::Edit.lanes().len());
    for (i, &expected) in GateScope::Edit.lanes().iter().enumerate() {
        assert_eq!(records[i].0, expected);
        assert!(matches!(records[i].1, LaneOutcome::Clean { .. }));
    }
}

#[test]
fn subdirectory_under_scope_dir_is_ignored() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);

    // A subdirectory named after a lane must not satisfy the missing-file
    // check; the lane is still treated as missing.
    fs::create_dir_all(dir.join("fmt")).unwrap();
    fs::write(dir.join("fmt").join("placeholder"), "ignored").unwrap();

    for lane in GateScope::Edit.lanes().iter().filter(|l| **l != Lane::Fmt) {
        fs::write(dir.join(format!("{}.json", lane_stem(*lane))), clean_artifact_json(*lane))
            .unwrap();
    }

    let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
    assert_missing_lane(&records, Lane::Fmt);
}

#[test]
fn unexpected_uppercase_stem_is_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);

    // A file with a capitalised stem is a different path on case-sensitive
    // filesystems and must be rejected as an unknown stem.
    fs::write(dir.join("Fmt.json"), clean_artifact_json(Lane::Fmt)).unwrap();

    let result = read_lane_artifacts(tmp.path(), GateScope::Edit);
    match result {
        Err(ReaderError::InputError { lane, cause }) => {
            assert_eq!(lane, GateScope::Edit.lanes()[0]);
            assert!(cause.contains("\"Fmt.json\""));
            assert!(cause.contains("does not name a known lane"));
        }
        other => panic!("expected InputError, got {other:?}"),
    }
}

#[test]
fn enumeration_is_deterministic_with_sorted_extra_file() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);

    // Write expected files in a deliberately non-sorted order, then add an
    // extra file whose name sorts between two lane stems. The reader must
    // reject the extra file deterministically by its actual name, regardless
    // of the order in which the artifacts were created on disk.
    let lane_order: Vec<Lane> = GateScope::Edit.lanes().iter().copied().rev().collect();
    for lane in &lane_order {
        fs::write(dir.join(format!("{}.json", lane_stem(*lane))), clean_artifact_json(*lane))
            .unwrap();
    }
    // `extra.json` sorts after every Edit lane stem.
    fs::write(dir.join("extra.json"), clean_artifact_json(Lane::Fmt)).unwrap();

    let result = read_lane_artifacts(tmp.path(), GateScope::Edit);
    match result {
        Err(ReaderError::InputError { cause, .. }) => {
            assert!(cause.contains("\"extra.json\""), "cause was: {cause}");
        }
        other => panic!("expected InputError, got {other:?}"),
    }
}

#[test]
fn scoped_lane_set_excludes_out_of_scope_lane() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Prepush);

    // Prepush scope does not include `Build`. A `build.json` placed in the
    // Prepush artifact directory must be rejected.
    for lane in GateScope::Prepush.lanes() {
        fs::write(dir.join(format!("{}.json", lane_stem(*lane))), clean_artifact_json(*lane))
            .unwrap();
    }
    fs::write(dir.join("build.json"), clean_artifact_json(Lane::Build)).unwrap();

    let result = read_lane_artifacts(tmp.path(), GateScope::Prepush);
    match result {
        Err(ReaderError::InputError { lane, cause }) => {
            assert_eq!(lane, Lane::Build);
            assert!(cause.contains("prepush"));
        }
        other => panic!("expected InputError, got {other:?}"),
    }
}

fn assert_missing_lane(records: &[(Lane, LaneOutcome)], expected: Lane) {
    let (_, outcome) = records.iter().find(|(lane, _)| *lane == expected).unwrap();
    match outcome {
        LaneOutcome::Failed { failure: LaneFailure::Infra { tool, reason } } => {
            assert_eq!(tool, expected.name());
            assert_eq!(reason, "output file missing");
        }
        other => panic!("expected missing-lane Infra failure for {expected}, got {other:?}"),
    }
}

/// Return the filename stem for a lane (mirrors artifact_reader internals).
fn lane_stem(lane: Lane) -> &'static str {
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
