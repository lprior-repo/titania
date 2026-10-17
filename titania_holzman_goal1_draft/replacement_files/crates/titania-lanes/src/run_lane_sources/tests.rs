//! Boundary tests for the actual source-discovery implementation.

use super::{SOURCE_WALK_LIMITS, SourceWalkError, SourceWalkResource, WalkLimits, collect_with_limits};
use std::path::PathBuf;

type TestResult = Result<(), Box<dyn std::error::Error>>;

fn assert_limit(
    result: Result<Vec<PathBuf>, SourceWalkError>,
    expected_resource: SourceWalkResource,
    expected_limit: usize,
) {
    match result {
        Err(SourceWalkError::LimitExceeded { resource, limit }) => {
            assert_eq!(resource, expected_resource);
            assert_eq!(limit, expected_limit);
        }
        other => panic!("expected {expected_resource:?} limit {expected_limit}, got {other:?}"),
    }
}

#[test]
fn sources_are_complete_sorted_and_keep_existing_exclusions() -> TestResult {
    let root = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join("src"))?;
    std::fs::create_dir(root.path().join("tests"))?;
    std::fs::create_dir(root.path().join("target"))?;
    std::fs::write(root.path().join("z.rs"), "")?;
    std::fs::write(root.path().join("src/a.rs"), "")?;
    std::fs::write(root.path().join("tests/excluded.rs"), "")?;
    std::fs::write(root.path().join("target/excluded.rs"), "")?;
    std::fs::write(root.path().join("build.rs"), "")?;
    std::fs::write(root.path().join("other.txt"), "")?;

    let actual = collect_with_limits(root.path(), SOURCE_WALK_LIMITS)?;
    assert_eq!(actual, vec![PathBuf::from("src/a.rs"), PathBuf::from("z.rs")]);
    Ok(())
}

#[test]
fn empty_root_completes_with_exactly_one_poll() -> TestResult {
    let root = tempfile::tempdir()?;
    let limits = WalkLimits { steps: 1, ..SOURCE_WALK_LIMITS };
    assert_eq!(collect_with_limits(root.path(), limits)?, Vec::<PathBuf>::new());
    Ok(())
}

#[test]
fn zero_poll_budget_rejects_before_opening_the_root() -> TestResult {
    let root = tempfile::tempdir()?;
    let missing = root.path().join("missing");
    let limits = WalkLimits { steps: 0, ..SOURCE_WALK_LIMITS };
    assert_limit(collect_with_limits(&missing, limits), SourceWalkResource::Steps, 0);
    Ok(())
}

#[test]
fn nonmatching_entries_still_exhaust_the_poll_budget() -> TestResult {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("a.txt"), "")?;
    std::fs::write(root.path().join("b.txt"), "")?;
    std::fs::write(root.path().join("c.txt"), "")?;
    let limits = WalkLimits { steps: 2, ..SOURCE_WALK_LIMITS };
    assert_limit(collect_with_limits(root.path(), limits), SourceWalkResource::Steps, 2);
    Ok(())
}

#[test]
fn collecting_the_last_file_is_not_evidence_of_end_of_directory() -> TestResult {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("a.rs"), "")?;
    let limits = WalkLimits { steps: 1, ..SOURCE_WALK_LIMITS };
    assert_limit(collect_with_limits(root.path(), limits), SourceWalkResource::Steps, 1);
    Ok(())
}

#[test]
fn one_file_and_the_final_eof_fit_exactly_two_polls() -> TestResult {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("a.rs"), "")?;
    let limits = WalkLimits { steps: 2, files: 1, ..SOURCE_WALK_LIMITS };
    assert_eq!(collect_with_limits(root.path(), limits)?, vec![PathBuf::from("a.rs")]);
    Ok(())
}

#[test]
fn file_limit_returns_error_instead_of_a_partial_inventory() -> TestResult {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("a.rs"), "")?;
    std::fs::write(root.path().join("b.rs"), "")?;
    let limits = WalkLimits { files: 1, ..SOURCE_WALK_LIMITS };
    assert_limit(collect_with_limits(root.path(), limits), SourceWalkResource::SourceFiles, 1);
    Ok(())
}

#[test]
fn directory_depth_includes_root_and_allows_its_exact_limit() -> TestResult {
    let root = tempfile::tempdir()?;
    std::fs::create_dir(root.path().join("src"))?;
    std::fs::write(root.path().join("src/a.rs"), "")?;
    let limits = WalkLimits { depth: 2, steps: 4, ..SOURCE_WALK_LIMITS };
    assert_eq!(collect_with_limits(root.path(), limits)?, vec![PathBuf::from("src/a.rs")]);
    Ok(())
}

#[test]
fn one_extra_directory_level_is_refused() -> TestResult {
    let root = tempfile::tempdir()?;
    std::fs::create_dir_all(root.path().join("a/b"))?;
    let limits = WalkLimits { depth: 2, ..SOURCE_WALK_LIMITS };
    assert_limit(collect_with_limits(root.path(), limits), SourceWalkResource::DirectoryDepth, 2);
    Ok(())
}

#[test]
fn zero_depth_is_refused_before_opening_the_root() -> TestResult {
    let root = tempfile::tempdir()?;
    let missing = root.path().join("missing");
    let limits = WalkLimits { depth: 0, ..SOURCE_WALK_LIMITS };
    assert_limit(collect_with_limits(&missing, limits), SourceWalkResource::DirectoryDepth, 0);
    Ok(())
}

#[test]
fn path_byte_limit_accepts_the_boundary_and_rejects_one_less() -> TestResult {
    let root = tempfile::tempdir()?;
    let file = root.path().join("a.rs");
    std::fs::write(&file, "")?;
    let bytes = file.as_os_str().as_encoded_bytes().len();
    let exact = WalkLimits { path_bytes: bytes, ..SOURCE_WALK_LIMITS };
    assert_eq!(collect_with_limits(root.path(), exact)?, vec![PathBuf::from("a.rs")]);
    let smaller = WalkLimits { path_bytes: bytes - 1, ..exact };
    assert_limit(collect_with_limits(root.path(), smaller), SourceWalkResource::PathBytes, bytes - 1);
    Ok(())
}

#[test]
fn cumulative_path_budget_is_checked_without_saturation() -> TestResult {
    let root = tempfile::tempdir()?;
    std::fs::write(root.path().join("a.rs"), "")?;
    let bytes = root.path().as_os_str().as_encoded_bytes().len() + "a.rs".len();
    let exact = WalkLimits { admitted_path_bytes: bytes, ..SOURCE_WALK_LIMITS };
    assert_eq!(collect_with_limits(root.path(), exact)?, vec![PathBuf::from("a.rs")]);
    let smaller = WalkLimits { admitted_path_bytes: bytes - 1, ..exact };
    assert_limit(
        collect_with_limits(root.path(), smaller),
        SourceWalkResource::AdmittedPathBytes,
        bytes - 1,
    );
    Ok(())
}

#[test]
fn missing_directory_preserves_its_io_failure() -> TestResult {
    let root = tempfile::tempdir()?;
    let missing = root.path().join("missing");
    match collect_with_limits(&missing, SOURCE_WALK_LIMITS) {
        Err(SourceWalkError::ReadDir { path, source }) => {
            assert_eq!(path, missing);
            assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
        }
        other => panic!("expected a typed directory-open failure, got {other:?}"),
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn stable_symlink_entries_are_not_followed() -> TestResult {
    let root = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    std::fs::write(outside.path().join("outside.rs"), "")?;
    std::os::unix::fs::symlink(outside.path(), root.path().join("linked"))?;
    assert_eq!(collect_with_limits(root.path(), SOURCE_WALK_LIMITS)?, Vec::<PathBuf>::new());
    Ok(())
}
