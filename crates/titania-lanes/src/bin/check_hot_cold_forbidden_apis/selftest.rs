use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
};

use crate::scan::{ScanError, scan};

pub(super) fn self_test() -> i32 {
    let root = fixture_root();
    if !prepare_fixture(&root) {
        return 1;
    }
    fixture_result(&root)
}

fn prepare_fixture(root: &Path) -> bool {
    if let Err(error) = reset_fixture(root) {
        return emit_failure(format_args!("FixtureFailure: cleanup failed: {error}"));
    }
    if let Err(error) = write_fixtures(root) {
        return emit_failure(format_args!("FixtureFailure: write failed: {error}"));
    }
    true
}

fn emit_failure(args: std::fmt::Arguments<'_>) -> bool {
    if crate::write_stderr_line(args).is_err() {
        return false;
    }
    false
}

fn fixture_result(root: &Path) -> i32 {
    match missing_required_classes(root) {
        Ok(missing) if missing.is_empty() => fixture_exit(&crate::write_stdout_line(format_args!(
            "FixturePass: hot/cold forbidden API scanner"
        ))),
        Ok(missing) => {
            let _result = crate::write_stderr_line(format_args!(
                "FixtureFailure: missing classes {missing:?}"
            ));
            1
        }
        Err(error) => {
            let _emitted = emit_failure(format_args!("FixtureFailure: scan failed: {error}"));
            1
        }
    }
}

const fn fixture_exit(result: &io::Result<()>) -> i32 {
    match result {
        Ok(()) => 0,
        Err(_error) => 1,
    }
}

fn fixture_root() -> PathBuf {
    std::env::temp_dir().join(format!("hot-cold-scan-{}", std::process::id()))
}

/// Remove any stale fixture directory.
///
/// # Errors
///
/// Returns filesystem errors other than a missing fixture directory.
fn reset_fixture(root: &Path) -> io::Result<()> {
    match fs::remove_dir_all(root) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Write one fixture source file, creating its parent directory first.
///
/// # Errors
///
/// Returns directory creation or file write errors.
fn write_fixture(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

/// Write the self-test fixture tree.
///
/// # Errors
///
/// Returns fixture file creation errors.
fn write_fixtures(root: &Path) -> io::Result<()> {
    let hot = root.join("crates/titania-core/src/engine.rs");
    let cold = root.join("crates/titania-core/src/diagnostic.rs");
    write_fixture(
        &hot,
        "pub fn bad() { println!(\"x\"); let _m: HashMap<String, u8> = HashMap::new(); let _c = std::sync::mpsc::channel(); }\n",
    )?;
    write_fixture(&cold, "pub fn ok() { println!(\"diagnostic only\"); }\n")
}

/// Return required finding classes not produced by the fixture scan.
///
/// # Errors
///
/// Returns scanner errors from the fixture scan.
fn missing_required_classes(root: &Path) -> Result<Vec<&'static str>, ScanError> {
    let (_classified, violations, _justified) = scan(root)?;
    let classes: BTreeSet<&'static str> =
        violations.iter().map(|finding| finding.class_id).collect();
    Ok(required_classes().iter().copied().filter(|class_id| !classes.contains(class_id)).collect())
}

const fn required_classes() -> [&'static str; 3] {
    ["FORMAT-PRINT-001", "MAP-STRING-001", "CHANNEL-UNBOUNDED-001"]
}
