//! Behavior tests for aggregate dispatch wiring.
//!
//! Bead: tn-cgk.2. These tests prove that the CLI reads existing lane
//! artifacts and emits a typed report instead of returning MissingImplementation.

use std::{env, fs, path::Path, process::Command};

use serde_json::Value;
use tempfile::TempDir;

fn binary() -> std::path::PathBuf {
    env::var("CARGO_BIN_EXE_titania-check")
        .expect("CARGO_BIN_EXE_titania-check not set; run via cargo test")
        .into()
}

fn run_in(root: &Path, args: &[&str]) -> std::process::Output {
    // Stub Moon via TITANIA_MOON_BIN so `Command::Check` does not invoke the
    // real moon binary (which would error on tempdirs without `.moon/`).
    // `/bin/true` exits 0 with any args, leaving the aggregate to classify the
    // pre-baked lane artifacts. Tests that exercise real Moon dispatch set
    // their own stub or clear this variable.
    Command::new(binary())
        .args(args)
        .current_dir(root)
        .env("TITANIA_MOON_BIN", "/bin/true")
        .output()
        .expect("failed to spawn titania-check")
}

fn clean_edit_workspace() -> TempDir {
    let temp = tempfile::tempdir().expect("tempdir must be created");
    let edit = temp.path().join(".titania").join("out").join("edit");
    fs::create_dir_all(&edit).expect("artifact dir must be created");
    GateArtifact::edit_lanes().iter().for_each(|artifact| artifact.write(&edit));
    temp
}

fn edit_workspace_without(missing_file: &str) -> TempDir {
    let temp = tempfile::tempdir().expect("tempdir must be created");
    let edit = temp.path().join(".titania").join("out").join("edit");
    fs::create_dir_all(&edit).expect("artifact dir must be created");
    GateArtifact::edit_lanes()
        .iter()
        .filter(|artifact| artifact.file != missing_file)
        .for_each(|artifact| artifact.write(&edit));
    temp
}

struct GateArtifact {
    lane: &'static str,
    file: &'static str,
}

impl GateArtifact {
    const fn edit_lanes() -> [Self; 7] {
        [
            Self { lane: "Fmt", file: "fmt" },
            Self { lane: "Compile", file: "compile" },
            Self { lane: "Clippy", file: "clippy" },
            Self { lane: "AstGrep", file: "ast-grep" },
            Self { lane: "Dylint", file: "dylint" },
            Self { lane: "PanicScan", file: "panic-scan" },
            Self { lane: "PolicyScan", file: "policy-scan" },
        ]
    }

    fn write(&self, edit: &Path) {
        fs::write(edit.join(self.file).with_extension("json"), self.json())
            .expect("lane artifact must be written");
    }

    fn json(&self) -> String {
        format!(
            r#"{{"lane":"{}","outcome":{{"Clean":{{"evidence":{{"command":{{"executable":"titania-check","argv":["titania-check","run-lane","{}"]}},"tool_version":"embedded-test","exit_status":{{"Exited":{{"code":0}}}},"parsed_result_digest":"0000000000000000000000000000000000000000000000000000000000000000"}}}}}}}}"#,
            self.lane, self.file
        )
    }
}

fn parse_stdout(output: &std::process::Output, expected_code: i32) -> Value {
    assert_eq!(
        output.status.code(),
        Some(expected_code),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(output.stderr.is_empty(), "aggregate should not write stderr on report output");
    serde_json::from_slice(&output.stdout).expect("stdout must be JSON report")
}

fn assert_pass_report(report: &Value) {
    assert_eq!(report["variant"], "Pass");
    assert_eq!(report["receipt"]["schema_version"], 1);
    assert_eq!(report["receipt"]["scope"], "Edit");
    assert_eq!(report["receipt"]["lanes"].as_array().expect("lanes array").len(), 7);
    assert_eq!(report["per_lane"].as_array().expect("per_lane array").len(), 7);
}

fn assert_missing_lane_report(report: &Value) {
    assert_eq!(report["variant"], "Reject");
    assert_eq!(report["code_findings"].as_array().expect("code findings").len(), 0);
    assert_eq!(report["gate_failures"].as_array().expect("gate failures").len(), 1);
    assert_eq!(report["gate_failures"][0]["InfraFailure"]["tool"], "Dylint");
    assert_eq!(report["gate_failures"][0]["InfraFailure"]["reason"], "output file missing");
    assert_eq!(report["per_lane"].as_array().expect("per_lane array").len(), 7);
}

#[test]
fn aggregate_cli_reads_edit_lane_outputs_and_emits_report_json() {
    let workspace = clean_edit_workspace();
    let output = run_in(workspace.path(), &["aggregate", "--scope", "edit", "--emit", "json"]);
    let report = parse_stdout(&output, 0);
    assert_pass_report(&report);
}

#[test]
fn check_clears_stale_outputs_before_aggregating_moon_results() {
    let workspace = clean_edit_workspace();
    let output = run_in(workspace.path(), &["--scope", "edit", "--emit", "json"]);
    let report = parse_stdout(&output, 1);
    assert_eq!(report["variant"], "Reject");
    assert_eq!(report["code_findings"].as_array().expect("code findings").len(), 0);
    assert_eq!(report["gate_failures"].as_array().expect("gate failures").len(), 7);
}

#[test]
fn aggregate_cli_records_missing_lane_output_as_infra_failure_report() {
    let workspace = edit_workspace_without("dylint");
    let output = run_in(workspace.path(), &["aggregate", "--scope", "edit", "--emit", "json"]);
    let report = parse_stdout(&output, 1);
    assert_missing_lane_report(&report);
}
