use std::{
    collections::BTreeSet,
    fs, io,
    path::{Path, PathBuf},
};

use crate::scan::scan;

pub fn self_test() -> i32 {
    let root = fixture_root();
    if let Err(error) = reset_fixture(&root) {
        eprintln!("FixtureFailure: cleanup failed: {error}");
        return 1;
    }
    if let Err(error) = write_fixtures(&root) {
        eprintln!("FixtureFailure: write failed: {error}");
        return 1;
    }
    match missing_required_classes(&root) {
        Ok(missing) if missing.is_empty() => {
            println!("FixturePass: hot/cold forbidden API scanner");
            0
        }
        Ok(missing) => {
            eprintln!("FixtureFailure: missing classes {missing:?}");
            1
        }
        Err(error) => {
            eprintln!("FixtureFailure: scan failed: {error}");
            1
        }
    }
}

fn fixture_root() -> PathBuf {
    std::env::temp_dir().join(format!("hot-cold-scan-{}", std::process::id()))
}

/// Removes the fixture root directory, treating a missing directory
/// as success.
///
/// # Errors
/// Returns the underlying `io::Error` if `fs::remove_dir_all` fails
/// for any reason other than the directory already being absent.
fn reset_fixture(root: &Path) -> io::Result<()> {
    match fs::remove_dir_all(root) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

/// Writes a single fixture file, creating parent directories as needed.
///
/// # Errors
/// Returns the underlying `io::Error` from `fs::create_dir_all`
/// (when `path.parent()` is `Some`) or `fs::write`.
fn write_fixture(path: &Path, text: &str) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, text)
}

/// Writes the two fixture files (hot + cold) under the fixture root.
///
/// # Errors
/// Returns the first I/O error from `write_fixture` for either the
/// hot `engine.rs` or cold `diagnostic.rs` fixture.
fn write_fixtures(root: &Path) -> io::Result<()> {
    let hot = root.join("crates/titania-core/src/engine.rs");
    let cold = root.join("crates/titania-core/src/diagnostic.rs");
    write_fixture(
        &hot,
        "pub fn bad() { println!(\"x\"); let _m: HashMap<String, u8> = HashMap::new(); let _c = std::sync::mpsc::channel(); }\n",
    )?;
    write_fixture(&cold, "pub fn ok() { println!(\"diagnostic only\"); }\n")
}

/// Scans the fixture and returns the required classes that did NOT
/// appear in the violations set.
///
/// # Errors
/// Returns the underlying `scan(root)` error string when fixture
/// scanning fails (allow file, hot source enumeration, or per-file
/// read errors).
fn missing_required_classes(root: &Path) -> Result<Vec<&'static str>, String> {
    let (_classified, violations, _justified) = scan(root)?;
    let classes: BTreeSet<&'static str> =
        violations.iter().map(|finding| finding.class_id).collect();
    Ok(required_classes().iter().copied().filter(|class_id| !classes.contains(class_id)).collect())
}

const fn required_classes() -> [&'static str; 3] {
    ["FORMAT-PRINT-001", "MAP-STRING-001", "CHANNEL-UNBOUNDED-001"]
}
