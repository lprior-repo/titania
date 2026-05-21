use std::process::Command;
use std::{fs, path::Path, path::PathBuf};
use xtask::evidence::parse_ai_release_document;

#[test]
fn ai_release_includes_ui_release_gates() {
    assert_eq!(
        write_required_negative_fixtures().map_err(|error| error.to_string()),
        Ok(())
    );
    let output = run_ai_release();
    assert_eq!(
        output.as_ref().map(|value| value.status.code()),
        Ok(Some(0))
    );
    let evidence = read_ai_release_evidence();
    assert_eq!(
        evidence
            .as_ref()
            .map_err(|_| ())
            .and_then(|text| parsed_subgates(text).map_err(|_| ())),
        Ok(expected_subgates())
    );
}

fn run_ai_release() -> Result<std::process::Output, String> {
    let mut command = xtask_command();
    command
        .current_dir(workspace_root())
        .args(["ai-release", "--bead", "vb-nf2u"])
        .output()
        .map_err(|error| error.to_string())
}

fn xtask_command() -> Command {
    let cargo_bin = Path::new(env!("CARGO_BIN_EXE_xtask"));
    if cargo_bin.exists() {
        Command::new(cargo_bin)
    } else {
        let mut command = Command::new("cargo");
        command.args(["xtask", "--"]);
        command
    }
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|path| path.join("velvet-ballistics"))
        .unwrap_or_else(|| PathBuf::from("."))
}

fn read_ai_release_evidence() -> Result<String, String> {
    std::fs::read_to_string(workspace_root().join(".evidence/vb-nf2u/ai-release.yaml"))
        .map_err(|error| error.to_string())
}

fn expected_subgates() -> Vec<String> {
    [
        "ui_snapshot",
        "layout_readability",
        "redaction",
        "negative_fixture",
        "deterministic_capture",
        "evidence_shape",
    ]
    .iter()
    .map(|value| value.to_string())
    .collect()
}

fn write_required_negative_fixtures() -> std::io::Result<()> {
    let workspace = workspace_root();
    let root = workspace.join("target/vb-nf2u-negative-fixtures");
    fs::create_dir_all(&root)?;
    fs::write(
        root.join("intentional_overlap_fixture.txt"),
        "fixture_id=intentional_overlap_fixture\nscreen_id=execution_overview\nfirst_control_id=run_button\nsecond_control_id=stop_button\nexpected_gate=layout\nexpected_code=layout_violation\noverlap_area_px=600\nbounds={ x: 10, y: 10, width: 100, height: 60 }\nactual_status=failed\n",
    )?;
    fs::write(
        root.join("intentional_secret_fixture.txt"),
        "fixture_id=intentional_secret_fixture\nexpected_gate=redaction\nexpected_code=redaction_violation\nactual_status=failed\n",
    )
}

fn parsed_subgates(text: &str) -> Result<Vec<String>, String> {
    let doc = parse_ai_release_document(text)?;
    Ok(doc
        .subgates
        .iter()
        .map(|subgate| subgate.as_str().to_string())
        .collect())
}
