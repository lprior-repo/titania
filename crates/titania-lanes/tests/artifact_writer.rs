//! Atomic lane-artifact writer — failing-first tests.
//!
//! These tests assert the *missing* public API that the v1 spec (§11.2)
//! requires: lane runners write a `LaneOutcome` as JSON to
//! `.titania/out/<scope>/<lane>.json` using atomic temp-file-then-rename,
//! creating parent directories as needed.
//!
//! Beads: tn-0i8.1, tn-rqo

use std::path::Path;

use tempfile::TempDir;
use titania_core::{
    CommandEvidence, Digest, Finding, GateScope, Lane, LaneEvidence, LaneOutcome, Location,
    RepairHint, RuleId, WorkspacePath,
};

/// Type alias used throughout this test module.
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

/// Helper: build a minimal, internally-consistent `LaneEvidence` for a
/// `LaneOutcome::Clean`.
fn stub_evidence() -> LaneEvidence {
    let command = CommandEvidence::new(
        "cargo".into(),
        vec!["cargo".into(), "fmt".into(), "--check".into()].into_boxed_slice(),
    )
    .unwrap();
    LaneEvidence::new(
        command,
        "rustfmt 1.84.0".into(),
        titania_core::ProcessTermination::Exited { code: 0 },
        Digest::from_bytes(b"stub-evidence-digest"),
    )
    .unwrap()
}

/// Helper: build a single `Finding` for a `LaneOutcome::Findings` path.
fn stub_finding() -> Finding {
    Finding::reject(
        Lane::Fmt,
        RuleId::new("FUNC_PRINT_STDOUT").unwrap(),
        Location::Span {
            file: WorkspacePath::new("src/main.rs").unwrap(),
            line_start: 42,
            col_start: 5,
            line_end: 42,
            col_end: 30,
        },
        "Found `println!` in production source".into(),
        RepairHint::RequiresHumanReview { note: "Replace with tracing or a logging facade".into() },
    )
}

/// Build a minimal synthetic target project inside a `TempDir` — a single
/// Cargo package with a `Cargo.toml` and a `src/lib.rs`.
fn make_target() -> Result<(TempDir, std::path::PathBuf), std::io::Error> {
    let tmp = TempDir::new()?;
    let root = tmp.path().to_path_buf();

    std::fs::write(
        root.join("Cargo.toml"),
        r#"
[package]
name = "synthetic-target"
version = "0.1.0"
edition = "2021"
"#,
    )?;

    std::fs::create_dir_all(root.join("src"))?;
    std::fs::write(root.join("src/lib.rs"), "")?;

    Ok((tmp, root))
}

// ---------------------------------------------------------------------------
// 1. Clean outcome — Happy path
// ---------------------------------------------------------------------------

#[test]
fn clean_outcome_written_to_scoped_lane_file() -> TestResult {
    let (_tmp, target) = make_target()?;
    let scope = GateScope::Edit;
    let lane = Lane::Fmt;
    let outcome = LaneOutcome::Clean { evidence: stub_evidence() };

    // API does not exist yet — this is the RED assertion.
    // The call should produce the expected file path inside the target root.
    let written =
        titania_lanes::artifact_writer::write_lane_artifact(&target, scope, lane, &outcome)?;

    let expected = target.join(".titania").join("out").join("edit").join("fmt.json");
    assert_eq!(written, expected, "returned path must match the scoped lane file");
    assert!(written.starts_with(&target), "written file must reside inside the target root");
    assert!(written.exists(), "lane artifact file must exist on disk after write");

    Ok(())
}

// ---------------------------------------------------------------------------
// 2. Findings outcome — Happy path
// ---------------------------------------------------------------------------

#[test]
fn findings_outcome_written_to_scoped_lane_file() -> TestResult {
    let (_tmp, target) = make_target()?;
    let scope = GateScope::Edit;
    let lane = Lane::Fmt;
    let outcome = LaneOutcome::Findings { findings: Box::new([stub_finding()]) };

    let written =
        titania_lanes::artifact_writer::write_lane_artifact(&target, scope, lane, &outcome)?;

    let expected = target.join(".titania").join("out").join("edit").join("fmt.json");
    assert_eq!(written, expected);
    assert!(written.exists());

    Ok(())
}

// ---------------------------------------------------------------------------
// 3. JSON payload contains the lane name
// ---------------------------------------------------------------------------

#[test]
fn written_artifact_json_contains_lane_name() -> TestResult {
    let (_tmp, target) = make_target()?;
    let scope = GateScope::Edit;
    let lane = Lane::Fmt;
    let outcome = LaneOutcome::Clean { evidence: stub_evidence() };

    let written =
        titania_lanes::artifact_writer::write_lane_artifact(&target, scope, lane, &outcome)?;

    let payload = std::fs::read_to_string(&written)?;
    let json: serde_json::Value = serde_json::from_str(&payload)?;

    assert!(json.get("lane").is_some(), "artifact JSON must contain a \"lane\" field");
    assert_eq!(
        json["lane"].as_str(),
        Some("Fmt"),
        "lane field must be PascalCase as per serde serialization"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// 4. Parent directories are created automatically
// ---------------------------------------------------------------------------

#[test]
fn parent_directories_created_on_write() -> TestResult {
    let (_tmp, target) = make_target()?;

    // The `.titania/out/edit/` tree does NOT exist yet.
    assert!(!target.join(".titania").exists(), ".titania/ must not exist before the write");

    let scope = GateScope::Edit;
    let lane = Lane::Fmt;
    let outcome = LaneOutcome::Clean { evidence: stub_evidence() };

    let written =
        titania_lanes::artifact_writer::write_lane_artifact(&target, scope, lane, &outcome)?;

    assert!(target.join(".titania").is_dir(), ".titania/ directory must exist after write");
    assert!(target.join(".titania/out").is_dir(), ".titania/out/ directory must exist after write");
    assert!(
        target.join(".titania/out/edit").is_dir(),
        ".titania/out/edit/ directory must exist after write"
    );
    assert!(written.exists());

    Ok(())
}

// ---------------------------------------------------------------------------
// 5. No temporary file remains after a successful write
// ---------------------------------------------------------------------------

#[test]
fn no_temp_file_remains_after_successful_write() -> TestResult {
    let (_tmp, target) = make_target()?;

    // Walk the future output directory *before* writing to establish a
    // baseline that no stray temp files exist.
    let pre_temp_count = |dir: &Path| {
        std::fs::read_dir(dir)
            .ok()
            .into_iter()
            .flatten()
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let ns = name.to_string_lossy();
                ns.starts_with(".titania-out-") && ns.ends_with(".tmp")
            })
            .count()
    };

    let scope = GateScope::Edit;
    let lane = Lane::Fmt;
    let outcome = LaneOutcome::Clean { evidence: stub_evidence() };

    let written =
        titania_lanes::artifact_writer::write_lane_artifact(&target, scope, lane, &outcome)?;

    // After a successful atomic write there must be zero .tmp files under
    // the output directory.
    let temp_count = pre_temp_count(&target.join(".titania/out/edit"));
    assert_eq!(temp_count, 0, "no temporary file should remain after atomic rename");

    // The final artifact must exist.
    assert!(written.exists());

    Ok(())
}

// ---------------------------------------------------------------------------
// 6. Lane identity matches across all edit lanes
// ---------------------------------------------------------------------------

#[test]
fn artifact_json_lane_matches_lane_enum_for_all_edit_lanes() -> TestResult {
    let scope = GateScope::Edit;
    let outcomes = [
        (Lane::Fmt, LaneOutcome::Clean { evidence: stub_evidence() }),
        (Lane::Compile, LaneOutcome::Clean { evidence: stub_evidence() }),
        (Lane::Clippy, LaneOutcome::Clean { evidence: stub_evidence() }),
        (Lane::AstGrep, LaneOutcome::Clean { evidence: stub_evidence() }),
        (Lane::Dylint, LaneOutcome::Clean { evidence: stub_evidence() }),
        (Lane::PanicScan, LaneOutcome::Clean { evidence: stub_evidence() }),
        (Lane::PolicyScan, LaneOutcome::Clean { evidence: stub_evidence() }),
    ];

    for (lane, outcome) in &outcomes {
        let (_tmp, target) = make_target()?;
        let written =
            titania_lanes::artifact_writer::write_lane_artifact(&target, scope, *lane, outcome)?;

        let payload = std::fs::read_to_string(&written)?;
        let json: serde_json::Value = serde_json::from_str(&payload)?;
        let lane_name = lane.name(); // PascalCase

        assert_eq!(
            json["lane"].as_str(),
            Some(lane_name),
            "lane field must match Lane::name() ({lane_name})"
        );
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// 7. Failure path — output root is a file, not a directory
// ---------------------------------------------------------------------------

#[test]
fn write_fails_when_output_root_is_not_a_directory() -> TestResult {
    let tmp = TempDir::new()?;
    let target = tmp.path().join("not-a-directory");
    std::fs::write(&target, "not-a-directory")?;

    let scope = GateScope::Edit;
    let lane = Lane::Fmt;
    let outcome = LaneOutcome::Clean { evidence: stub_evidence() };

    // The API should return an error rather than panicking.
    let result =
        titania_lanes::artifact_writer::write_lane_artifact(&target, scope, lane, &outcome);

    assert!(result.is_err(), "writing with a non-directory output root must fail");

    Ok(())
}

// ---------------------------------------------------------------------------
// 8. Failure path — output root does not exist at all
// ---------------------------------------------------------------------------

#[test]
fn write_fails_when_output_root_does_not_exist() -> TestResult {
    let tmp = TempDir::new()?;
    let nonexistent = tmp.path().join("no-such-dir-xyz");

    assert!(!nonexistent.exists(), "test precondition: path must not exist");

    let scope = GateScope::Edit;
    let lane = Lane::Fmt;
    let outcome = LaneOutcome::Clean { evidence: stub_evidence() };

    let result =
        titania_lanes::artifact_writer::write_lane_artifact(&nonexistent, scope, lane, &outcome);

    assert!(result.is_err(), "writing to a non-existent output root must fail");

    Ok(())
}

// ---------------------------------------------------------------------------
// 9. Findings artifact includes outcome variant in JSON
// ---------------------------------------------------------------------------

#[test]
fn findings_artifact_includes_outcome_variant_in_json() -> TestResult {
    let (_tmp, target) = make_target()?;
    let scope = GateScope::Edit;
    let lane = Lane::Fmt;
    let outcome = LaneOutcome::Findings { findings: Box::new([stub_finding()]) };

    let written =
        titania_lanes::artifact_writer::write_lane_artifact(&target, scope, lane, &outcome)?;

    let payload = std::fs::read_to_string(&written)?;
    let json: serde_json::Value = serde_json::from_str(&payload)?;

    assert!(json.get("outcome").is_some(), "artifact JSON must contain an \"outcome\" field");
    assert_eq!(
        json["outcome"]["variant"].as_str(),
        Some("findings"),
        "outcome variant must be \"findings\" for LaneOutcome::Findings"
    );

    Ok(())
}

// ---------------------------------------------------------------------------
// 10. Clean artifact includes outcome variant in JSON
// ---------------------------------------------------------------------------

#[test]
fn clean_artifact_includes_outcome_variant_in_json() -> TestResult {
    let (_tmp, target) = make_target()?;
    let scope = GateScope::Edit;
    let lane = Lane::Fmt;
    let outcome = LaneOutcome::Clean { evidence: stub_evidence() };

    let written =
        titania_lanes::artifact_writer::write_lane_artifact(&target, scope, lane, &outcome)?;

    let payload = std::fs::read_to_string(&written)?;
    let json: serde_json::Value = serde_json::from_str(&payload)?;

    assert_eq!(
        json["outcome"]["variant"].as_str(),
        Some("clean"),
        "outcome variant must be \"clean\" for LaneOutcome::Clean"
    );

    Ok(())
}
