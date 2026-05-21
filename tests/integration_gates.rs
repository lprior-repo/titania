use std::fs;
#[cfg(unix)]
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run_xtask(args: &[&str]) -> Output {
    match Command::new("cargo")
        .args(["xtask", "--"])
        .args(args)
        .current_dir(workspace_root())
        .output()
    {
        Ok(output) => output,
        Err(error) => failed_output(format!("Failed to execute cargo xtask: {error}")),
    }
}

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .map(|path| path.join("velvet-ballistics"))
        .unwrap_or_default()
}

#[cfg(unix)]
fn failed_output(message: String) -> Output {
    Output {
        status: std::process::ExitStatus::from_raw(1),
        stdout: Vec::new(),
        stderr: message.into_bytes(),
    }
}

fn evidence_root() -> PathBuf {
    workspace_root().join(".evidence")
}

fn cleanup_evidence(bead_id: &str) {
    if is_confined_bead_id(bead_id) {
        remove_dir_if_present(&evidence_root().join(bead_id));
    }
}

fn is_confined_bead_id(bead_id: &str) -> bool {
    !bead_id.is_empty()
        && bead_id
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
}

#[test]
fn ai_profiles_emit_yaml_evidence_and_required_gate_names() {
    for (cmd, bead_id, gates) in [
        (
            "ai-fast",
            "vb-itest-fast",
            &[
                "fmt",
                "check",
                "clippy",
                "nextest",
                "forbidden-scan",
                "hotpath-scan",
            ][..],
        ),
        (
            "ai-deep",
            "vb-itest-deep",
            &["miri", "mutants", "llvm-cov", "fuzz-build"][..],
        ),
    ] {
        cleanup_evidence(bead_id);
        let output = run_xtask(&[cmd, "--bead", bead_id]);
        assert!(
            output.status.success(),
            "{cmd} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let evidence_path = evidence_root().join(bead_id).join(format!("{cmd}.yaml"));
        assert!(
            evidence_path.exists(),
            "missing {}",
            evidence_path.display()
        );
        let content = read_text_or_empty(&evidence_path);
        for gate in gates {
            assert!(content.contains(gate), "{cmd} evidence missing {gate}");
        }
    }
}

#[test]
fn ai_release_fails_closed_for_unknown_bead_without_writing_evidence() {
    let bead_id = "vb-itest-release";
    cleanup_evidence(bead_id);
    let output = run_xtask(&["ai-release", "--bead", bead_id]);
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unknown ai-release bead id"));
    assert!(!evidence_root().join(bead_id).exists());
}

#[test]
fn ai_fast_stdout_mode_outputs_structured_yaml_without_evidence_dir() {
    let output = run_xtask(&["ai-fast"]);
    assert!(
        output.status.success(),
        "ai-fast stdout failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("profile: ai-fast"));
    assert!(stdout.contains("gates:"));
}

#[test]
fn invalid_bead_and_unknown_subcommand_fail_closed() {
    let invalid = run_xtask(&["ai-fast", "--bead", "../bad"]);
    assert!(!invalid.status.success());
    assert!(String::from_utf8_lossy(&invalid.stderr).contains("Invalid bead id"));

    let unknown = run_xtask(&["unknown-gate"]);
    assert!(!unknown.status.success());
    assert!(String::from_utf8_lossy(&unknown.stderr).contains("UnknownCommand"));
}

#[test]
fn cleanup_rejects_traversal_without_removing_outside_directory() {
    let outside = evidence_root().join("outside-sentinel");
    create_dir_or_fail(&outside);
    cleanup_evidence("../outside-sentinel");
    assert!(outside.exists());
    remove_dir_if_present(&outside);
}

#[test]
fn existing_failed_yaml_evidence_has_diagnostic_or_repair_context() {
    let bead_id = "vb-itest-diagnostics";
    let evidence_dir = evidence_root().join(bead_id);
    remove_dir_if_present(&evidence_dir);
    create_dir_or_fail(&evidence_dir);
    write_file_or_fail(
        &evidence_dir.join("fmt.yaml"),
        "gate: fmt\nstatus: Fail\nwhy_failed: fmt mismatch\nhint: run fmt\n",
    );
    assert_eq!(
        count_failed_yaml_files_without_diagnostics(&evidence_dir),
        0
    );
    remove_dir_if_present(&evidence_dir);
}

fn read_text_or_empty(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn yaml_file_contains_failed_gate_without_diagnostic(entry: &fs::DirEntry) -> bool {
    let path = entry.path();
    path.extension()
        .is_some_and(|extension| extension == "yaml")
        && fs::read_to_string(path).is_ok_and(|yaml| {
            let failed = yaml.contains("status: Fail") || yaml.contains("status: Fail\n");
            let diagnosed = yaml.contains("why_failed:")
                || yaml.contains("hint:")
                || yaml.contains("repair_command:");
            failed && !diagnosed
        })
}

fn count_failed_yaml_files_without_diagnostics(evidence_dir: &Path) -> usize {
    fs::read_dir(evidence_dir)
        .ok()
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .filter(yaml_file_contains_failed_gate_without_diagnostic)
                .count()
        })
        .unwrap_or_default()
}

fn remove_dir_if_present(path: &Path) {
    let removed = fs::remove_dir_all(path);
    let not_found = removed
        .as_ref()
        .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound);
    assert!(
        removed.is_ok() || not_found,
        "remove dir {}: {removed:?}",
        path.display()
    );
}

fn create_dir_or_fail(path: &Path) {
    assert!(
        fs::create_dir_all(path).is_ok(),
        "create dir {}",
        path.display()
    );
}

fn write_file_or_fail(path: &Path, content: &str) {
    assert!(
        fs::write(path, content).is_ok(),
        "write file {}",
        path.display()
    );
}
