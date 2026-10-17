use std::{path::PathBuf, process::Command};

fn dylint_crate_path() -> PathBuf {
    let lanes_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    lanes_dir.join("..").join("titania-dylint")
}

fn fixtures_dir() -> PathBuf {
    let lanes_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    lanes_dir.join("tests").join("fixtures").join("dylint")
}

fn read_fixture(name: &str) -> String {
    let path = fixtures_dir().join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("cannot read fixture {name}: {error}"))
}

fn cargo_dylint_available() -> bool {
    Command::new("cargo")
        .args(["dylint", "--help"])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn write_fixture_workspace(root: &std::path::Path, lib_source: &str) {
    std::fs::create_dir_all(root.join("src")).expect("must create src dir");
    let manifest = format!(
        r#"[package]
name = "dylint-reject-repair-fixture"
version = "0.1.0"
edition = "2021"

[lib]
path = "src/lib.rs"

[workspace]
[workspace.metadata.dylint]
libraries = [{{ path = "{dylint}" }}]
"#,
        dylint = dylint_crate_path().display()
    );
    std::fs::write(root.join("Cargo.toml"), manifest).expect("must write Cargo.toml");
    std::fs::write(root.join("src").join("lib.rs"), lib_source).expect("must write lib.rs");
}

fn run_dylint(root: &std::path::Path) -> (i32, String) {
    let output = Command::new("cargo")
        .args(["dylint", "--workspace", "--all", "--", "--lib"])
        .current_dir(root)
        .output()
        .expect("must spawn cargo dylint");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}\n{stderr}");
    (output.status.code().unwrap_or(-1), combined)
}

#[test]
fn dylint_rejects_bypass_pub_allow_then_clean_after_repair() {
    if !cargo_dylint_available() {
        eprintln!("skip: cargo-dylint not installed in this environment");
        return;
    }

    let temp = tempfile::tempdir().expect("must create temp dir");
    let root = temp.path();

    write_fixture_workspace(root, &read_fixture("bypass_pub_allow_violation.rs"));
    let (reject_code, reject_output) = run_dylint(root);

    assert_ne!(
        reject_code, 0,
        "violation fixture must cause cargo-dylint to exit non-zero; output:\n{reject_output}"
    );
    assert!(
        reject_output.contains("BYPASS_PUB_ALLOW"),
        "output must name the BYPASS_PUB_ALLOW rule; got:\n{reject_output}"
    );
    assert!(
        reject_output.contains("#[allow(dead_code)]"),
        "output must point at the offending #[allow(dead_code)] attribute; got:\n{reject_output}"
    );
    assert!(
        reject_output.contains("src/lib.rs"),
        "output must reference the fixture source location; got:\n{reject_output}"
    );

    write_fixture_workspace(root, &read_fixture("bypass_pub_allow_clean.rs"));
    let (clean_code, clean_output) = run_dylint(root);

    assert_eq!(
        clean_code, 0,
        "clean fixture must let cargo-dylint exit 0; output:\n{clean_output}"
    );
    assert!(
        !clean_output.contains("BYPASS_"),
        "clean fixture must produce no BYPASS_* findings; got:\n{clean_output}"
    );
}
