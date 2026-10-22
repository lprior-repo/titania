use std::fs;

use tempfile::TempDir;
use titania_aggregate::{ReaderError, read_lane_artifacts};
use titania_core::{GateScope, Lane, LaneFailure, LaneOutcome, SkipReason};

fn clean_artifact_json(lane: Lane) -> String {
    format!(
        r#"{{"lane":"{}","outcome":{{"Clean":{{"evidence":{{"command":{{"executable":"cargo","argv":["cargo","check"]}},"tool_version":"1.0","exit_status":{{"Exited":{{"code":0}}}},"parsed_result_digest":"0000000000000000000000000000000000000000000000000000000000000000"}}}}}}}}"#,
        lane.name()
    )
}

fn skipped_artifact_json(lane: Lane) -> String {
    format!(r#"{{"lane":"{}","outcome":{{"Skipped":"NotSelectedByScope"}}}}"#, lane.name())
}

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

    let lane_order: Vec<Lane> = GateScope::Edit.lanes().iter().copied().rev().collect();
    for lane in &lane_order {
        fs::write(dir.join(format!("{}.json", lane_stem(*lane))), clean_artifact_json(*lane))
            .unwrap();
    }
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

fn failed_tool_artifact_json(lane: Lane, tool: &str) -> String {
    format!(
        r#"{{"lane":"{}","outcome":{{"Failed":{{"ToolFailure":{{"tool":"{tool}","termination":{{"Exited":{{"code":101}}}}}}}}}}}}"#,
        lane.name()
    )
}

fn findings_artifact_json(lane: Lane) -> String {
    format!(
        r#"{{"lane":"{}","outcome":{{"Findings":[{{"lane":"{}","rule_id":"RUSTC_TYPE_ERROR","location":"Workspace","message":"mismatched types","repair":{{"RequiresHumanReview":{{"note":"fix the type error"}}}},"effect":"Reject"}}]}}}}"#,
        lane.name(),
        lane.name()
    )
}

fn findings_artifact_json_with_rule(lane: Lane, rule_id: &str) -> String {
    format!(
        r#"{{"lane":"{}","outcome":{{"Findings":[{{"lane":"{}","rule_id":"{rule_id}","location":"Workspace","message":"recorded finding","repair":{{"RequiresHumanReview":{{"note":"address the finding"}}}},"effect":"Reject"}}]}}}}"#,
        lane.name(),
        lane.name()
    )
}

fn write_clean_except(dir: &std::path::Path, scope: GateScope, skip: &[Lane]) {
    for lane in scope.lanes() {
        if !skip.contains(lane) {
            fs::write(dir.join(format!("{}.json", lane_stem(*lane))), clean_artifact_json(*lane))
                .unwrap();
        }
    }
}

fn outcome_of<'a>(records: &'a [(Lane, LaneOutcome)], lane: Lane) -> &'a LaneOutcome {
    records
        .iter()
        .find(|(record_lane, _)| *record_lane == lane)
        .map(|(_, outcome)| outcome)
        .unwrap()
}

fn assert_prior_compilation_skip(records: &[(Lane, LaneOutcome)], lane: Lane) {
    match outcome_of(records, lane) {
        LaneOutcome::Skipped { reason: SkipReason::PriorCompilationFailure } => {}
        other => panic!("expected PriorCompilationFailure skip for {lane}, got {other:?}"),
    }
}

fn assert_recorded_findings(records: &[(Lane, LaneOutcome)], lane: Lane, rule_id: &str) {
    match outcome_of(records, lane) {
        LaneOutcome::Findings { findings } => {
            assert_eq!(findings.len(), 1, "{lane} must carry exactly its recorded finding");
            assert_eq!(findings[0].rule_id().as_str(), rule_id);
            assert!(findings[0].is_reject(), "recorded finding must keep its Reject effect");
        }
        other => panic!("expected recorded Findings for {lane}, got {other:?}"),
    }
}

#[test]
fn compile_failure_turns_missing_clippy_into_prior_compilation_skip() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);
    write_clean_except(&dir, GateScope::Edit, &[Lane::Compile, Lane::Clippy]);
    fs::write(dir.join("compile.json"), failed_tool_artifact_json(Lane::Compile, "cargo-check"))
        .unwrap();

    let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
    assert_prior_compilation_skip(&records, Lane::Clippy);
    assert!(
        matches!(outcome_of(&records, Lane::Compile), LaneOutcome::Failed { .. }),
        "compile outcome must stay Failed"
    );
    assert!(
        matches!(outcome_of(&records, Lane::AstGrep), LaneOutcome::Clean { .. }),
        "non-dependent lanes must stand as they ran"
    );
}

#[test]
fn compile_failure_skips_clippy_test_build_regardless_of_their_artifacts() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Release);
    write_clean_except(&dir, GateScope::Release, &[Lane::Compile, Lane::Test, Lane::Build]);
    fs::write(dir.join("compile.json"), failed_tool_artifact_json(Lane::Compile, "cargo-check"))
        .unwrap();
    fs::write(dir.join("test.json"), failed_tool_artifact_json(Lane::Test, "cargo-test")).unwrap();

    let records = read_lane_artifacts(tmp.path(), GateScope::Release).unwrap();
    assert_prior_compilation_skip(&records, Lane::Clippy);
    assert_prior_compilation_skip(&records, Lane::Test);
    assert_prior_compilation_skip(&records, Lane::Build);
    assert!(
        matches!(outcome_of(&records, Lane::Deny), LaneOutcome::Clean { .. }),
        "Deny does not depend on compilation and must stand as it ran"
    );
}

#[test]
fn compile_findings_turn_present_clippy_into_prior_compilation_skip() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);
    write_clean_except(&dir, GateScope::Edit, &[Lane::Compile]);
    fs::write(dir.join("compile.json"), findings_artifact_json(Lane::Compile)).unwrap();

    let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
    assert_prior_compilation_skip(&records, Lane::Clippy);
    assert!(
        matches!(outcome_of(&records, Lane::Compile), LaneOutcome::Findings { .. }),
        "compile outcome must stay Findings"
    );
}

#[test]
fn compile_failure_preserves_recorded_clippy_findings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Prepush);
    write_clean_except(&dir, GateScope::Prepush, &[Lane::Compile, Lane::Clippy, Lane::Test]);
    fs::write(dir.join("compile.json"), failed_tool_artifact_json(Lane::Compile, "cargo-check"))
        .unwrap();
    fs::write(
        dir.join("clippy.json"),
        findings_artifact_json_with_rule(Lane::Clippy, "CLIPPY_UNWRAP_USED"),
    )
    .unwrap();

    let records = read_lane_artifacts(tmp.path(), GateScope::Prepush).unwrap();
    assert_recorded_findings(&records, Lane::Clippy, "CLIPPY_UNWRAP_USED");
    assert_prior_compilation_skip(&records, Lane::Test);
    assert!(
        matches!(outcome_of(&records, Lane::Compile), LaneOutcome::Failed { .. }),
        "compile outcome must stay Failed"
    );
}

#[test]
fn compile_findings_preserve_recorded_clippy_findings() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);
    write_clean_except(&dir, GateScope::Edit, &[Lane::Compile, Lane::Clippy]);
    fs::write(dir.join("compile.json"), findings_artifact_json(Lane::Compile)).unwrap();
    fs::write(
        dir.join("clippy.json"),
        findings_artifact_json_with_rule(Lane::Clippy, "CLIPPY_UNWRAP_USED"),
    )
    .unwrap();

    let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
    assert_recorded_findings(&records, Lane::Clippy, "CLIPPY_UNWRAP_USED");
    assert!(
        matches!(outcome_of(&records, Lane::Compile), LaneOutcome::Findings { .. }),
        "compile outcome must stay Findings"
    );
}

#[test]
fn clean_compile_keeps_missing_clippy_as_infra_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);
    write_clean_except(&dir, GateScope::Edit, &[Lane::Clippy]);

    let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
    assert_missing_lane(&records, Lane::Clippy);
}

#[test]
fn missing_compile_keeps_missing_clippy_as_infra_failure() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = setup_scope_dir(&tmp, GateScope::Edit);
    write_clean_except(&dir, GateScope::Edit, &[Lane::Compile, Lane::Clippy]);

    let records = read_lane_artifacts(tmp.path(), GateScope::Edit).unwrap();
    assert_missing_lane(&records, Lane::Compile);
    assert_missing_lane(&records, Lane::Clippy);
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
        Lane::Kani => "kani",
        Lane::Mutants => "mutants",
    }
}
