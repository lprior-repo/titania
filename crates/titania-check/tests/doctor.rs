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
    run_with_path_and_cwd(args, path, None)
}

/// Run the doctor binary with an optional override `PATH` and working dir.
fn run_with_path_and_cwd(
    args: &[&str],
    path: Option<&str>,
    cwd: Option<&std::path::Path>,
) -> (i32, String, String) {
    let mut command = Command::new(binary());
    let _ = command.args(args);
    if let Some(path) = path {
        let _ = command.env("PATH", path);
    }
    if let Some(cwd) = cwd {
        let _ = command.current_dir(cwd);
    }
    let output = command
        .env("LD_LIBRARY_PATH", "")
        .env("DYLD_LIBRARY_PATH", "")
        .output()
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
/// Parse a single human doctor row into `[Tool, Required, Installed, Version, Path]`.
///
/// Columns are separated by runs of two or more whitespace characters, so the
/// split is robust to column-width changes — only the inter-column gap matters,
/// not its exact width. Required-column labels with an internal space (e.g.
/// `no (edit)`) survive because the gap before/after the label is wider than
/// the single space inside it. The trailing `Path` column is taken verbatim so
/// paths containing spaces are preserved.
fn parse_row(row: &str) -> Option<Vec<&str>> {
    let trimmed = row.trim();
    if trimmed.is_empty() || trimmed.starts_with("titania-check") || trimmed.starts_with("Status:")
    {
        return None;
    }
    let bytes = trimmed.as_bytes();
    let mut cols: Vec<&str> = Vec::new();
    let mut start = 0usize;
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i].is_ascii_whitespace() {
            let mut j = i;
            while j < bytes.len() && bytes[j].is_ascii_whitespace() {
                j += 1;
            }
            if j - i >= 2 {
                cols.push(&trimmed[start..i]);
                start = j;
            }
            i = j;
        } else {
            i += 1;
        }
    }
    cols.push(&trimmed[start..]);
    if cols.len() < 5 {
        return None;
    }
    let mut out: Vec<&str> = cols[..4].iter().map(|s| s.trim()).collect();
    out.extend(cols[4..].iter().map(|s| s.trim()));
    Some(out)
}

/// Locate a tool row in human doctor output and return its columns.
fn row_columns<'a>(stdout: &'a str, tool: &str) -> Option<Vec<&'a str>> {
    stdout.lines().find_map(|line| {
        let cols = parse_row(line)?;
        (cols.first() == Some(&tool)).then_some(cols)
    })
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
    // v1 §12: clippy-driver row is labeled `clippy` in the Tool column.
    assert!(stdout.contains("clippy"), "missing clippy row: {stdout}");
    assert!(stdout.contains("ast-grep"), "missing ast-grep row: {stdout}");
    // v1 §12: cargo-dylint row is labeled `dylint` in the Tool column.
    assert!(stdout.contains("dylint"), "missing dylint row: {stdout}");
    // The library row keeps an internal label so ABI state is visible.
    assert!(stdout.contains("dylint-lib"), "missing dylint-lib row: {stdout}");
    assert!(stdout.contains("Status:"), "missing status: {stdout}");
}

/// v1 §12 edit-scope row labels: `embedded` (ast-grep) in Required and `—`
/// in Installed, `no (edit)` (cargo-deny), and `optional` (sccache).
#[test]
fn doctor_human_required_labels_match_v1_spec_edit_scope() {
    let (_code, stdout, _stderr) = run(&["doctor", "--scope", "edit"]);
    // Required / Installed column labels per tool — checked by parsing each
    // row into `[Tool, Required, Installed, Version, Path]` so the assertion
    // is independent of column widths.
    let ast_grep = row_columns(&stdout, "ast-grep")
        .unwrap_or_else(|| panic!("missing ast-grep row in edit scope: {stdout}"));
    assert_eq!(
        ast_grep[1], "embedded",
        "ast-grep Required column must read `embedded` (v1 §12); row={ast_grep:?}"
    );
    assert_eq!(
        ast_grep[2], "—",
        "ast-grep Installed column must read `—` (v1 §12); row={ast_grep:?}"
    );
    let cargo_deny = row_columns(&stdout, "cargo-deny")
        .unwrap_or_else(|| panic!("missing cargo-deny row in edit scope: {stdout}"));
    assert_eq!(
        cargo_deny[1], "no (edit)",
        "cargo-deny Required column must read `no (edit)` in edit scope (v1 §12); row={cargo_deny:?}"
    );
    let sccache = row_columns(&stdout, "sccache")
        .unwrap_or_else(|| panic!("missing sccache row in edit scope: {stdout}"));
    assert_eq!(
        sccache[1], "optional",
        "sccache Required column must read `optional` (v1 §12); row={sccache:?}"
    );
}
/// In prepush/release scope, cargo-deny is required: the Required column must
/// read `yes`, not `no (edit)`.
#[test]
fn doctor_human_cargo_deny_required_label_flips_outside_edit() {
    let (_prepush_code, prepush_stdout, _stderr) = run(&["doctor", "--scope", "prepush"]);
    let (_release_code, release_stdout, _stderr) = run(&["doctor", "--scope", "release"]);
    let prepush_deny = row_columns(&prepush_stdout, "cargo-deny")
        .unwrap_or_else(|| panic!("missing cargo-deny row in prepush scope: {prepush_stdout}"));
    assert_eq!(
        prepush_deny[1], "yes",
        "cargo-deny Required column must read `yes` in prepush scope; row={prepush_deny:?}"
    );
    let release_deny = row_columns(&release_stdout, "cargo-deny")
        .unwrap_or_else(|| panic!("missing cargo-deny row in release scope: {release_stdout}"));
    assert_eq!(
        release_deny[1], "yes",
        "cargo-deny Required column must read `yes` in release scope; row={release_deny:?}"
    );
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

#[cfg(unix)]
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
#[cfg(unix)]
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

/// H1 reconciliation: when `cargo-dylint` and `dylint-link` are installed and
/// the workspace has dylint metadata naming titania, but no pre-built
/// `.so`/`.dylib` exists on disk, the doctor must report the library as
/// `metadata/built-on-demand` (installed=true).
///
/// `dylint-link` is required because cargo-dylint 6.0.1 uses it to link the
/// cdylib during a metadata-mode build. Without it, the "built-on-demand"
/// claim would be dishonest.
#[cfg(unix)]
#[test]
fn doctor_metadata_mode_reports_library_built_on_demand() {
    let tmp = tempfile::tempdir().expect("must create temp dir");
    let bin_dir = tmp.path();

    // Fake cargo-dylint so the subcommand probe reports it as installed.
    let dylint_bin = bin_dir.join("cargo-dylint");
    std::fs::write(&dylint_bin, "#!/bin/sh\necho \"cargo-dylint 0.0.0\"\n")
        .expect("must write fake cargo-dylint");
    std::fs::set_permissions(&dylint_bin, std::fs::Permissions::from_mode(0o755))
        .expect("must chmod fake cargo-dylint");

    // Fake dylint-link so metadata mode can honestly report built-on-demand.
    let dylint_link_bin = bin_dir.join("dylint-link");
    std::fs::write(&dylint_link_bin, "#!/bin/sh\necho \"dylint-link 0.0.0\"\n")
        .expect("must write fake dylint-link");
    std::fs::set_permissions(&dylint_link_bin, std::fs::Permissions::from_mode(0o755))
        .expect("must chmod fake dylint-link");

    // A workspace root whose Cargo.toml configures the titania dylint library
    // via `[workspace.metadata.dylint]`, mirroring titania's own manifest.
    let ws_dir = bin_dir.join("workspace");
    std::fs::create_dir_all(ws_dir.join("src")).expect("must create workspace src");
    std::fs::write(
        ws_dir.join("Cargo.toml"),
        "[package]\n\
         name = \"ws\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         \n\
         [lib]\n\
         path = \"src/lib.rs\"\n\
         \n\
         [workspace]\n\
         [workspace.metadata.dylint]\n\
         libraries = [{ path = \"crates/titania-dylint\" }]\n",
    )
    .expect("must write workspace Cargo.toml");
    std::fs::write(ws_dir.join("src").join("lib.rs"), "").expect("must write empty lib.rs");

    let path = bin_dir.to_string_lossy();
    let (_code, stdout, stderr) = run_with_path_and_cwd(
        &["doctor", "--scope", "edit", "--emit", "json"],
        Some(&path),
        Some(&ws_dir),
    );
    assert!(stderr.is_empty(), "doctor must not write stderr: {stderr}");

    let report: Value =
        serde_json::from_str(&stdout).expect("doctor JSON must be parseable: {stdout}");
    let lib_row = tool(&report, "libtitania_dylint");
    assert_eq!(
        lib_row["installed"], true,
        "metadata mode must report dylint-lib as installed: {lib_row:#}"
    );
    assert_eq!(
        lib_row["version"].as_str(),
        Some("metadata/built-on-demand"),
        "metadata mode version must be 'metadata/built-on-demand': {lib_row:#}"
    );
    // The library must NOT appear in missing_required.
    let missing = report["missing_required"].as_array().expect("missing_required must be an array");
    assert!(
        !missing.iter().any(|n| n == "libtitania_dylint"),
        "metadata-mode dylint-lib must not be in missing_required: {report:#}"
    );
}

/// H1 negative case: when `cargo-dylint` and `dylint-link` are installed but
/// the workspace has NO dylint metadata and no pre-built library, the doctor
/// must report `abi:unknown` (installed=false) — the metadata fix must not
/// over-report.
#[cfg(unix)]
#[test]
fn doctor_no_metadata_and_no_lib_reports_abi_unknown() {
    let tmp = tempfile::tempdir().expect("must create temp dir");
    let bin_dir = tmp.path();

    let dylint_bin = bin_dir.join("cargo-dylint");
    std::fs::write(&dylint_bin, "#!/bin/sh\necho \"cargo-dylint 0.0.0\"\n")
        .expect("must write fake cargo-dylint");
    std::fs::set_permissions(&dylint_bin, std::fs::Permissions::from_mode(0o755))
        .expect("must chmod fake cargo-dylint");

    // Fake dylint-link so the probe reaches the metadata check (rather than
    // the dylint-link-missing branch) and correctly reports abi:unknown.
    let dylint_link_bin = bin_dir.join("dylint-link");
    std::fs::write(&dylint_link_bin, "#!/bin/sh\necho \"dylint-link 0.0.0\"\n")
        .expect("must write fake dylint-link");
    std::fs::set_permissions(&dylint_link_bin, std::fs::Permissions::from_mode(0o755))
        .expect("must chmod fake dylint-link");

    // Workspace WITHOUT dylint metadata.
    let ws_dir = bin_dir.join("workspace");
    std::fs::create_dir_all(ws_dir.join("src")).expect("must create workspace src");
    std::fs::write(
        ws_dir.join("Cargo.toml"),
        "[package]\nname = \"ws\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[lib]\npath = \"src/lib.rs\"\n\n[workspace]\n",
    )
    .expect("must write workspace Cargo.toml");
    std::fs::write(ws_dir.join("src").join("lib.rs"), "").expect("must write empty lib.rs");

    let path = bin_dir.to_string_lossy();
    let (_code, stdout, stderr) = run_with_path_and_cwd(
        &["doctor", "--scope", "edit", "--emit", "json"],
        Some(&path),
        Some(&ws_dir),
    );
    assert!(stderr.is_empty(), "doctor must not write stderr: {stderr}");

    let report: Value =
        serde_json::from_str(&stdout).expect("doctor JSON must be parseable: {stdout}");
    let lib_row = tool(&report, "libtitania_dylint");
    assert_eq!(
        lib_row["installed"], false,
        "no metadata + no lib must report installed=false: {lib_row:#}"
    );
    assert_eq!(
        lib_row["version"].as_str(),
        Some("abi:unknown"),
        "no metadata + no lib version must be abi:unknown: {lib_row:#}"
    );
}

/// dylint-link absent + no prebuilt lib → library missing, dylint-link in
/// missing_required, exit 3. This is the core H1 fix: the doctor must NOT
/// overclaim "metadata/built-on-demand" when dylint-link is absent, because
/// cargo-dylint 6.0.1 genuinely cannot build the cdylib without it.
#[cfg(unix)]
#[test]
fn doctor_dylint_link_absent_no_prebuilt_lib_reports_missing() {
    let tmp = tempfile::tempdir().expect("must create temp dir");
    let bin_dir = tmp.path();

    // Fake cargo-dylint (present).
    let dylint_bin = bin_dir.join("cargo-dylint");
    std::fs::write(&dylint_bin, "#!/bin/sh\necho \"cargo-dylint 0.0.0\"\n")
        .expect("must write fake cargo-dylint");
    std::fs::set_permissions(&dylint_bin, std::fs::Permissions::from_mode(0o755))
        .expect("must chmod fake cargo-dylint");

    // No dylint-link created — it is intentionally absent.
    // No prebuilt lib created.

    // Workspace WITH dylint metadata (to prove metadata alone is insufficient).
    let ws_dir = bin_dir.join("workspace");
    std::fs::create_dir_all(ws_dir.join("src")).expect("must create workspace src");
    std::fs::write(
        ws_dir.join("Cargo.toml"),
        "[package]\nname = \"ws\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n\
         [lib]\npath = \"src/lib.rs\"\n\n\
         [workspace]\n[workspace.metadata.dylint]\n\
         libraries = [{ path = \"crates/titania-dylint\" }]\n",
    )
    .expect("must write workspace Cargo.toml");
    std::fs::write(ws_dir.join("src").join("lib.rs"), "").expect("must write empty lib.rs");

    let path = bin_dir.to_string_lossy();
    let (code, stdout, stderr) = run_with_path_and_cwd(
        &["doctor", "--scope", "edit", "--emit", "json"],
        Some(&path),
        Some(&ws_dir),
    );
    assert!(stderr.is_empty(), "doctor must not write stderr: {stderr}");
    assert_eq!(code, 3, "dylint-link missing must trigger MissingRequiredTools (exit 3)");

    let report: Value =
        serde_json::from_str(&stdout).expect("doctor JSON must be parseable: {stdout}");
    assert_eq!(report["status"], "MissingRequiredTools");

    let missing = report["missing_required"].as_array().expect("missing_required array");
    assert!(
        missing.iter().any(|n| n == "dylint-link"),
        "dylint-link must be in missing_required: {report:#}"
    );

    let lib_row = tool(&report, "libtitania_dylint");
    assert_eq!(
        lib_row["installed"], false,
        "lib must be installed=false when dylint-link is absent: {lib_row:#}"
    );
    assert_eq!(
        lib_row["version"].as_str(),
        Some("dylint-link missing (required to build libtitania_dylint from source)"),
        "lib version must name dylint-link as the blocker: {lib_row:#}"
    );
}

/// Prebuilt sibling lib present → library available regardless of dylint-link.
/// A consumer that ships `libtitania_dylint.so` via binstall does not need
/// `dylint-link` because the cdylib is already built.
#[cfg(unix)]
#[test]
fn doctor_prebuilt_lib_available_regardless_of_dylint_link() {
    let tmp = tempfile::tempdir().expect("must create temp dir");
    let bin_dir = tmp.path();

    // Fake cargo-dylint (present).
    let dylint_bin = bin_dir.join("cargo-dylint");
    std::fs::write(&dylint_bin, "#!/bin/sh\necho \"cargo-dylint 0.0.0\"\n")
        .expect("must write fake cargo-dylint");
    std::fs::set_permissions(&dylint_bin, std::fs::Permissions::from_mode(0o755))
        .expect("must chmod fake cargo-dylint");

    // No dylint-link created — intentionally absent.

    // Prebuilt sibling lib with valid ELF header + ABI markers.
    let lib = bin_dir.join("libtitania_dylint.so");
    std::fs::write(&lib, b"\x7fELFsynthetic-dylint-fixture\0dylint_version\0register_lints\0")
        .expect("must write fake lib");

    let path = bin_dir.to_string_lossy();
    let (_code, stdout, stderr) =
        run_with_path(&["doctor", "--scope", "edit", "--emit", "json"], Some(&path));
    assert!(stderr.is_empty(), "doctor must not write stderr: {stderr}");

    let report: Value =
        serde_json::from_str(&stdout).expect("doctor JSON must be parseable: {stdout}");

    let lib_row = tool(&report, "libtitania_dylint");
    assert_eq!(
        lib_row["installed"], true,
        "prebuilt lib must report installed=true regardless of dylint-link: {lib_row:#}"
    );
    assert_eq!(
        lib_row["version"].as_str(),
        Some("abi:verified"),
        "prebuilt lib version must be abi:verified: {lib_row:#}"
    );

    // dylint-link must NOT be in missing_required (not needed when prebuilt lib exists).
    let missing = report["missing_required"].as_array().expect("missing_required array");
    assert!(
        !missing.iter().any(|n| n == "dylint-link"),
        "dylint-link must not be required when prebuilt lib exists: {report:#}"
    );

    // dylint-link must not be required when prebuilt lib exists.
    let dylint_link_row = tool(&report, "dylint-link");
    assert_eq!(
        dylint_link_row["required"], false,
        "dylint-link must not be required when prebuilt lib exists: {dylint_link_row:#}"
    );
}
