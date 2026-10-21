use std::{env, fs, path::Path, process::Command};

use serde_json::Value;
use tempfile::TempDir;

const MALFORMED_POLICY: &str = "[lints\nthis is not toml";

const VALID_POLICY: &str = "\
[architecture]
core_dirs = [\"src/core\"]
infra_crates = [\"tokio\"]
";

const VALID_EXCEPTIONS: &str = "\
[[exceptions]]
rule_id = \"BYPASS_EXPECT_ATTR\"
path = \"crates/titania-dylint/src/lib.rs\"
owner = \"titania-maintainers\"
reason = \"policy error cli fixture\"
expires_on = \"2999-01-01\"
review = \"policy-error-cli\"
";

const EXPIRED_EXCEPTIONS: &str = "\
[[exceptions]]
rule_id = \"BYPASS_EXPECT_ATTR\"
path = \"crates/titania-dylint/src/lib.rs\"
owner = \"titania-maintainers\"
reason = \"policy error cli fixture\"
expires_on = \"2020-01-01\"
review = \"policy-error-cli\"
";

fn binary() -> std::path::PathBuf {
    env::var("CARGO_BIN_EXE_titania-check")
        .expect("CARGO_BIN_EXE_titania-check not set; run via cargo test")
        .into()
}

fn run_in(root: &Path, args: &[&str]) -> std::process::Output {
    let mut cmd = Command::new(binary());
    let _ = cmd.args(args);
    let _ = cmd.current_dir(root);
    #[cfg(unix)]
    let _ = cmd.env("TITANIA_MOON_BIN", "/bin/true");
    cmd.output().expect("failed to spawn titania-check")
}

fn workspace_with_profile(policy: Option<&str>, exceptions: Option<&str>) -> TempDir {
    let temp = tempfile::tempdir().expect("tempdir must be created");
    let edit = temp.path().join(".titania").join("out").join("edit");
    fs::create_dir_all(&edit).expect("artifact dir must be created");
    GateArtifact::edit_lanes().iter().for_each(|artifact| artifact.write(&edit));
    let profile = temp.path().join(".titania").join("profiles").join("strict-ai");
    fs::create_dir_all(&profile).expect("profile dir must be created");
    if let Some(content) = policy {
        fs::write(profile.join("policy.toml"), content).expect("policy must be written");
    }
    if let Some(content) = exceptions {
        fs::write(profile.join("exceptions.toml"), content).expect("exceptions must be written");
    }
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
    assert!(output.stderr.is_empty(), "report output should not write stderr");
    serde_json::from_slice(&output.stdout).expect("stdout must be JSON report")
}

fn diagnostic_messages(report: &Value) -> Vec<String> {
    report["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|diagnostic| diagnostic["message"].as_str().expect("diagnostic message").to_owned())
        .collect()
}

#[test]
fn aggregate_with_malformed_policy_emits_policy_error_and_exit_two() {
    let workspace = workspace_with_profile(Some(MALFORMED_POLICY), Some(VALID_EXCEPTIONS));
    let output = run_in(workspace.path(), &["aggregate", "--scope", "edit", "--emit", "json"]);
    let report = parse_stdout(&output, 2);
    assert_eq!(report["variant"], "PolicyError");
    let diagnostics = report["diagnostics"].as_array().expect("diagnostics array");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["severity"], "Error");
    assert_eq!(diagnostics[0]["file"], ".titania/profiles/strict-ai/policy.toml");
}

#[test]
fn aggregate_with_malformed_exceptions_emits_policy_error_and_exit_two() {
    let workspace = workspace_with_profile(Some(VALID_POLICY), Some("not toml either"));
    let output = run_in(workspace.path(), &["aggregate", "--scope", "edit", "--emit", "json"]);
    let report = parse_stdout(&output, 2);
    assert_eq!(report["variant"], "PolicyError");
    let diagnostics = report["diagnostics"].as_array().expect("diagnostics array");
    assert_eq!(diagnostics.len(), 1);
    assert_eq!(diagnostics[0]["severity"], "Error");
    assert_eq!(diagnostics[0]["file"], ".titania/profiles/strict-ai/exceptions.toml");
    assert!(
        diagnostic_messages(&report)
            .iter()
            .any(|message| message.contains("POLICY_EXCEPTION_PARSE_ERROR")),
        "diagnostics must carry the parse error code"
    );
}

#[test]
fn aggregate_with_expired_exception_emits_policy_error_with_expired_code() {
    let workspace = workspace_with_profile(Some(VALID_POLICY), Some(EXPIRED_EXCEPTIONS));
    let output = run_in(workspace.path(), &["aggregate", "--scope", "edit", "--emit", "json"]);
    let report = parse_stdout(&output, 2);
    assert_eq!(report["variant"], "PolicyError");
    assert_eq!(report["diagnostics"].as_array().expect("diagnostics array").len(), 1);
    assert!(
        diagnostic_messages(&report)
            .iter()
            .any(|message| message.contains("POLICY_EXCEPTION_EXPIRED")),
        "diagnostics must carry the exact POLICY_EXCEPTION_EXPIRED rule id"
    );
}

#[test]
fn aggregate_with_valid_policy_still_passes() {
    let workspace = workspace_with_profile(Some(VALID_POLICY), Some(VALID_EXCEPTIONS));
    let output = run_in(workspace.path(), &["aggregate", "--scope", "edit", "--emit", "json"]);
    let report = parse_stdout(&output, 0);
    assert_eq!(report["variant"], "Pass");
    assert_eq!(report["receipt"]["schema_version"], 1);
    assert_eq!(report["receipt"]["scope"], "Edit");
    assert_eq!(report["per_lane"].as_array().expect("per_lane array").len(), 7);
}

#[cfg(unix)]
#[test]
fn check_with_malformed_policy_emits_policy_error_and_exit_two() {
    let workspace = workspace_with_profile(Some(MALFORMED_POLICY), None);
    let output = run_in(workspace.path(), &["--scope", "edit", "--emit", "json"]);
    let report = parse_stdout(&output, 2);
    assert_eq!(report["variant"], "PolicyError");
    assert_eq!(report["diagnostics"].as_array().expect("diagnostics array").len(), 1);
}
