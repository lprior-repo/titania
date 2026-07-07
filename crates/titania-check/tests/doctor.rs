//! Doctor behavior tests for bead tn-4rq.2.

use serde_json::Value;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::process::Command;

fn binary() -> std::path::PathBuf {
    std::env::var("CARGO_BIN_EXE_titania-check")
        .expect("CARGO_BIN_EXE_titania-check not set")
        .into()
}

fn run(args: &[&str]) -> (i32, String, String) {
    run_with_path(args, None)
}

fn run_with_path(args: &[&str], path: Option<&str>) -> (i32, String, String) {
    let output = match path {
        Some(path) => Command::new(binary())
            .args(args)
            .env("PATH", path)
            .env("LD_LIBRARY_PATH", "")
            .env("DYLD_LIBRARY_PATH", "")
            .output(),
        None => Command::new(binary()).args(args).output(),
    }
    .expect("failed to execute titania-check");
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (output.status.code().unwrap_or(-1), stdout, stderr)
}

fn doctor_json(scope: &str) -> (i32, Value, String) {
    let (code, stdout, stderr) = run(&["doctor", "--scope", scope, "--emit", "json"]);
    let parsed = serde_json::from_str(&stdout).expect("doctor JSON must be parseable");
    (code, parsed, stderr)
}

fn tool<'a>(report: &'a Value, name: &str) -> &'a Value {
    report["tools"]
        .as_array()
        .expect("tools must be an array")
        .iter()
        .find(|tool| tool["name"] == name)
        .unwrap_or_else(|| panic!("missing tool row {name}; report={report:#}"))
}

#[test]
fn doctor_human_output_contains_contract_columns_and_rows() {
    let (code, stdout, stderr) = run(&["doctor", "--scope", "edit"]);
    assert!(stderr.is_empty(), "doctor must not write stderr: {stderr}");
    assert!(code == 0 || code == 3, "doctor exit code must be 0 or 3, got: {code}");
    assert!(stdout.contains("titania-check doctor — scope: edit"), "missing header: {stdout}");
    assert!(stdout.contains("Tool"), "missing Tool column: {stdout}");
    assert!(stdout.contains("Required"), "missing Required column: {stdout}");
    assert!(stdout.contains("Installed"), "missing Installed column: {stdout}");
    assert!(stdout.contains("Version"), "missing Version column: {stdout}");
    assert!(stdout.contains("Path"), "missing Path column: {stdout}");
    assert!(stdout.contains("moon"), "missing moon row: {stdout}");
    assert!(stdout.contains("cargo"), "missing cargo row: {stdout}");
    assert!(stdout.contains("rustfmt"), "missing rustfmt row: {stdout}");
    assert!(stdout.contains("clippy-driver"), "missing clippy row: {stdout}");
    assert!(stdout.contains("ast-grep"), "missing ast-grep row: {stdout}");
    assert!(stdout.contains("cargo-dylint"), "missing cargo-dylint row: {stdout}");
    assert!(stdout.contains("libtitania_dylint"), "missing dylint library row: {stdout}");
    assert!(stdout.contains("Status:"), "missing status: {stdout}");
}

#[test]
fn doctor_json_edit_scope_contains_required_tool_matrix() {
    let (code, report, stderr) = doctor_json("edit");
    assert!(stderr.is_empty(), "doctor must not write stderr: {stderr}");
    assert!(code == 0 || code == 3, "doctor exit code must be 0 or 3, got: {code}");
    assert_eq!(report["scope"], "edit");
    assert!(report["tools"].is_array(), "tools must be an array: {report:#}");
    assert!(report["missing_required"].is_array(), "missing_required must be an array");
    assert!(report["status"] == "OK" || report["status"] == "MissingRequiredTools");

    assert_eq!(tool(&report, "moon")["required"], true);
    assert_eq!(tool(&report, "cargo")["required"], true);
    assert_eq!(tool(&report, "rustfmt")["required"], true);
    assert_eq!(tool(&report, "clippy-driver")["required"], true);
    assert_eq!(tool(&report, "cargo-dylint")["required"], true);
    assert_eq!(tool(&report, "libtitania_dylint")["required"], true);
    assert_eq!(tool(&report, "cargo-deny")["required"], false);
    assert_eq!(tool(&report, "sccache")["required"], false);
}

#[test]
fn doctor_json_ast_grep_is_embedded_without_path_or_version() {
    let (_code, report, _stderr) = doctor_json("edit");
    let ast_grep = tool(&report, "ast-grep");
    assert_eq!(ast_grep["required"], true);
    assert_eq!(ast_grep["installed"], true);
    assert!(ast_grep.get("embedded").is_none(), "doctor JSON contract has no embedded field");
    assert!(ast_grep["version"].is_null(), "embedded ast-grep has no external version");
    assert!(ast_grep["path"].is_null(), "embedded ast-grep has no external path");
}

#[test]
fn doctor_json_cargo_deny_required_only_for_prepush_and_release() {
    let (_edit_code, edit, _edit_stderr) = doctor_json("edit");
    let (_prepush_code, prepush, _prepush_stderr) = doctor_json("prepush");
    let (_release_code, release, _release_stderr) = doctor_json("release");
    assert_eq!(tool(&edit, "cargo-deny")["required"], false);
    assert_eq!(tool(&prepush, "cargo-deny")["required"], true);
    assert_eq!(tool(&release, "cargo-deny")["required"], true);
}

#[test]
fn doctor_empty_path_reports_missing_required_tools_exit_3() {
    let (code, stdout, stderr) =
        run_with_path(&["doctor", "--scope", "edit", "--emit", "json"], Some(""));
    assert!(stderr.is_empty(), "doctor must not write stderr: {stderr}");
    assert_eq!(code, 3, "missing required tools must exit 3");
    let report: Value = serde_json::from_str(&stdout).expect("doctor JSON must be parseable");
    assert_eq!(report["status"], "MissingRequiredTools");

    let missing = report["missing_required"].as_array().expect("missing_required array");
    // edit scope requires PATH tools; libtitania_dylint is only required when cargo-dylint exists.
    for required in ["moon", "cargo", "rustfmt", "clippy-driver", "cargo-dylint"] {
        assert!(missing.iter().any(|n| n == required), "{required} must be missing: {report:#}");
    }
    // edit scope does NOT require cargo-deny
    assert!(
        !missing.iter().any(|n| n == "cargo-deny"),
        "cargo-deny must not be in missing_required for edit: {report:#}"
    );
    // optional sccache must never appear in missing_required
    assert!(
        !missing.iter().any(|n| n == "sccache"),
        "optional sccache must not be in missing_required: {report:#}"
    );
    // libtitania_dylint follows the cargo-dylint row; absent cargo-dylint means
    // the library row is informational and must not be in missing_required.
    assert!(
        !missing.iter().any(|n| n == "libtitania_dylint"),
        "libtitania_dylint must not be required when cargo-dylint is absent: {report:#}"
    );
    let dylint_lib = tool(&report, "libtitania_dylint");
    assert_eq!(dylint_lib["required"], false);

    // ast-grep is embedded in titania-check, so it remains installed with empty PATH.
    let ast_grep = tool(&report, "ast-grep");
    assert_eq!(ast_grep["installed"], true);
    assert!(ast_grep.get("embedded").is_none(), "doctor JSON contract has no embedded field");
}

#[test]
fn doctor_empty_path_prepush_release_require_cargo_deny() {
    for scope in ["prepush", "release"] {
        let (code, stdout, stderr) =
            run_with_path(&["doctor", "--scope", scope, "--emit", "json"], Some(""));
        assert!(stderr.is_empty(), "doctor must not write stderr: {stderr}");
        assert_eq!(code, 3, "{scope} missing required tools must exit 3");
        let report: Value = serde_json::from_str(&stdout).expect("doctor JSON must be parseable");
        assert_eq!(report["status"], "MissingRequiredTools");

        let missing = report["missing_required"].as_array().expect("missing_required array");
        // prepush/release both require cargo-deny
        assert!(
            missing.iter().any(|n| n == "cargo-deny"),
            "{scope} must require cargo-deny: {report:#}"
        );
        // optional sccache must never appear
        assert!(
            !missing.iter().any(|n| n == "sccache"),
            "{scope}: optional sccache must not be in missing_required: {report:#}"
        );
    }
}

#[test]
fn doctor_abi_mismatch_yields_missing_required_library() {
    // Build a temp dir containing a fake cargo-dylint (shell script) and an
    // incompatible libtitania_dylint.so (plain file with no ELF header).
    let tmp = tempfile::tempdir().expect("must create temp dir");
    let bin_dir = tmp.path();

    // Fake cargo-dylint that exits 0 with a version line.
    let dylint_bin = bin_dir.join("cargo-dylint");
    std::fs::write(&dylint_bin, "#!/bin/sh\necho \"cargo-dylint 0.0.0\"\n")
        .expect("must write fake cargo-dylint");
    #[cfg(unix)]
    std::fs::set_permissions(&dylint_bin, std::fs::Permissions::from_mode(0o755))
        .expect("must chmod fake");

    // Incompatible library: text with both Dylint marker names but no dynamic
    // library header, so `abi_is_compatible` must still reject it.
    let fake_lib = bin_dir.join("libtitania_dylint.so");
    std::fs::write(&fake_lib, "dylint_version\nregister_lints\n")
        .expect("must write fake libtitania_dylint");

    let path = bin_dir.to_string_lossy();

    // Run doctor with the crafted PATH.
    let (code, stdout, stderr) =
        run_with_path(&["doctor", "--scope", "edit", "--emit", "json"], Some(&path));
    assert!(stderr.is_empty(), "doctor must not write stderr: {stderr}");

    // ABI mismatch → libtitania_dylint is required but not installed → exit 3.
    assert_eq!(code, 3, "ABI mismatch must exit 3");
    let report: Value = serde_json::from_str(&stdout).expect("doctor JSON must be parseable");
    assert_eq!(report["status"], "MissingRequiredTools");

    let missing = report["missing_required"].as_array().expect("missing_required array");
    assert!(
        missing.iter().any(|n| n == "libtitania_dylint"),
        "libtitania_dylint must be in missing_required on ABI mismatch: {report:#}"
    );

    // The library row must NOT claim installed=true.
    let lib_row = tool(&report, "libtitania_dylint");
    assert_eq!(lib_row["installed"], false, "incompatible lib must show installed=false");
    assert_eq!(
        lib_row["version"].as_str(),
        Some("abi:mismatch"),
        "incompatible lib version must be abi:mismatch"
    );
    // cargo-dylint itself should appear installed (we provided a working script).
    let cargo_dylint = tool(&report, "cargo-dylint");
    assert_eq!(cargo_dylint["installed"], true, "fake cargo-dylint should be installed");
}

/// Prove the ABI probe works without `nm` on the PATH using a self-contained
/// dynamic-library-header plus marker fixture.
#[test]
fn doctor_abi_probe_no_nm_dependency() {
    let tmp = tempfile::tempdir().expect("must create temp dir");
    let bin_dir = tmp.path();

    let dest_lib = bin_dir.join("libtitania_dylint.so");
    std::fs::write(&dest_lib, b"\x7fELFsynthetic-dylint-fixture\0dylint_version\0register_lints\0")
        .expect("must write marker library fixture");
    // Fake cargo-dylint (so it appears installed).
    let dylint_bin = bin_dir.join("cargo-dylint");
    std::fs::write(&dylint_bin, "#!/bin/sh\necho \"cargo-dylint 0.0.0\"\n")
        .expect("must write fake cargo-dylint");
    #[cfg(unix)]
    std::fs::set_permissions(&dylint_bin, std::fs::Permissions::from_mode(0o755))
        .expect("must chmod fake");

    // Run with PATH pointing to our temp dir and NO nm in PATH.
    let path = bin_dir.to_string_lossy();
    let (_code, stdout, stderr) =
        run_with_path(&["doctor", "--scope", "edit", "--emit", "json"], Some(&path));
    assert!(stderr.is_empty(), "doctor must not write stderr: {stderr}");

    // Marker fixture has both ABI markers → installed=true, no missing_required.
    let report: Value = serde_json::from_str(&stdout).expect("doctor JSON must be parseable");
    let lib_row = tool(&report, "libtitania_dylint");
    assert_eq!(lib_row["installed"], true, "marker fixture must show installed=true");
    assert_eq!(
        lib_row["version"].as_str(),
        Some("abi:verified"),
        "marker fixture version must be abi:verified"
    );

    let missing = report["missing_required"].as_array().expect("missing_required array");
    assert!(
        !missing.iter().any(|n| n == "libtitania_dylint"),
        "marker fixture must not be in missing_required: {report:#}"
    );
}

#[test]
fn doctor_unknown_scope_remains_input_error() {
    let (code, stdout, stderr) = run(&["doctor", "--scope", "full"]);
    assert_eq!(code, 3, "unknown scope must be InputError");
    assert!(stdout.is_empty(), "unknown scope must not write stdout: {stdout}");
    assert!(stderr.contains("unknown scope"), "stderr must name unknown scope: {stderr}");
    assert!(stderr.contains("full"), "stderr must include rejected scope: {stderr}");
}
