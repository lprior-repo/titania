//! Public API tests for target project discovery.
//!
//! These tests cover the pure core boundary: every constructor validates
//! typed observations supplied by the caller. The shell layer (in
//! `titania-lanes`) owns the filesystem reads and feeds observations into
//! the same selectors these tests exercise directly.

use std::path::{Path, PathBuf};

use camino::Utf8Path;
use titania_core::{
    ManifestKind, TargetObservation, TargetProject, TargetProjectError, select_target_root,
};

fn cargo_manifest(name: &str) -> String {
    format!("[package]\nname = \"{name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n")
}

fn observed_dir(path: &Path, manifest: ManifestKind) -> TargetObservation {
    let utf8 = Utf8Path::from_path(path).expect("test path must be UTF-8");
    TargetObservation::new(utf8.to_path_buf(), true, manifest)
}

#[test]
fn target_project_public_api_reports_exact_shape_errors() {
    let empty = TargetProject::try_from_path_string(Utf8Path::new("")).unwrap_err();
    assert_eq!(empty, TargetProjectError::Empty);

    let relative = TargetProject::try_from_path_string(Utf8Path::new("relative/path")).unwrap_err();
    assert_eq!(relative, TargetProjectError::NonAbsolute("relative/path".to_owned()));
}

#[test]
fn target_project_public_api_rejects_relative_observation() {
    let obs = TargetObservation::new(
        Utf8Path::new("relative/path").to_path_buf(),
        true,
        ManifestKind::File,
    );
    let err = TargetProject::try_from_observation(&obs).unwrap_err();
    assert_eq!(err, TargetProjectError::NonAbsolute("relative/path".to_owned()));
}

#[test]
fn target_project_public_api_rejects_non_existent_directory_observation() {
    let obs = TargetObservation::new(
        Utf8Path::new("/nonexistent/titania").to_path_buf(),
        false,
        ManifestKind::Missing,
    );
    let err = TargetProject::try_from_observation(&obs).unwrap_err();
    assert_eq!(err, TargetProjectError::NotFound);
}

#[test]
fn target_project_public_api_rejects_manifest_directory_observation() {
    let obs = TargetObservation::new(
        Utf8Path::new("/tmp/x").to_path_buf(),
        true,
        ManifestKind::Directory,
    );
    let err = TargetProject::try_from_observation(&obs).unwrap_err();
    assert_eq!(err, TargetProjectError::CargoTomlNotFile);
}

#[test]
fn target_project_public_api_rejects_missing_manifest_observation() {
    let obs =
        TargetObservation::new(Utf8Path::new("/tmp/x").to_path_buf(), true, ManifestKind::Missing);
    let err = TargetProject::try_from_observation(&obs).unwrap_err();
    assert_eq!(err, TargetProjectError::NoCargoToml);
}

#[test]
fn target_project_public_api_json_round_trips_absolute_root() {
    let root = Utf8Path::new("/tmp/json_root").to_path_buf();
    let target = TargetProject::try_from_path_string(&root).unwrap();

    let json = serde_json::to_string(&target).unwrap();
    let back: TargetProject = serde_json::from_str(&json).unwrap();

    assert_eq!(target, back);
}

#[test]
fn target_project_public_api_json_rejects_relative_root() {
    let relative = "\"src/foo\"";
    let err: Result<TargetProject, _> = serde_json::from_str(relative);
    assert!(err.is_err());
}

// ---------------------------------------------------------------------------
// Pure selector tests
// ---------------------------------------------------------------------------

#[test]
fn select_target_root_returns_workspace_when_present() {
    let cwd = Utf8Path::new("/repo/sub");
    let outer = Utf8Path::new("/repo").to_path_buf();
    let inner = cwd.to_path_buf();
    let observations = vec![
        observed_dir(inner.as_std_path(), ManifestKind::File),
        observed_dir(outer.as_std_path(), ManifestKind::File),
    ];
    let manifests = vec![
        titania_core::ManifestObservation {
            root: inner.clone(),
            manifest_path: inner.join("Cargo.toml"),
            status: titania_core::ManifestStatus::Package,
        },
        titania_core::ManifestObservation {
            root: outer.clone(),
            manifest_path: outer.join("Cargo.toml"),
            status: titania_core::ManifestStatus::Workspace,
        },
    ];

    let selected = select_target_root(&observations, &manifests).unwrap();
    assert_eq!(selected, outer);
}

#[test]
fn select_target_root_falls_back_to_package_when_no_workspace() {
    let cwd = Utf8Path::new("/repo/sub");
    let inner = cwd.to_path_buf();
    let observations = vec![observed_dir(inner.as_std_path(), ManifestKind::File)];
    let manifests = vec![titania_core::ManifestObservation {
        root: inner.clone(),
        manifest_path: inner.join("Cargo.toml"),
        status: titania_core::ManifestStatus::Package,
    }];

    let selected = select_target_root(&observations, &manifests).unwrap();
    assert_eq!(selected, inner);
}

#[test]
fn select_target_root_ignores_non_workspace_tables() {
    let outer = Utf8Path::new("/repo").to_path_buf();
    let inner = Utf8Path::new("/repo/sub").to_path_buf();
    let observations = vec![
        observed_dir(inner.as_std_path(), ManifestKind::File),
        observed_dir(outer.as_std_path(), ManifestKind::File),
    ];
    let manifests = vec![
        titania_core::ManifestObservation {
            root: inner.clone(),
            manifest_path: inner.join("Cargo.toml"),
            status: titania_core::ManifestStatus::Package,
        },
        titania_core::ManifestObservation {
            root: outer.clone(),
            manifest_path: outer.join("Cargo.toml"),
            status: titania_core::ManifestStatus::Other,
        },
    ];

    let selected = select_target_root(&observations, &manifests).unwrap();
    assert_eq!(selected, inner);
}

#[test]
fn select_target_root_rejects_malformed_selected_manifest() {
    let root = Utf8Path::new("/repo/broken").to_path_buf();
    let observations = vec![observed_dir(root.as_std_path(), ManifestKind::File)];
    let manifests = vec![titania_core::ManifestObservation {
        root: root.clone(),
        manifest_path: root.join("Cargo.toml"),
        status: titania_core::ManifestStatus::Malformed,
    }];

    let err = select_target_root(&observations, &manifests).unwrap_err();
    assert_eq!(
        err,
        TargetProjectError::MalformedCargoToml { path: root.join("Cargo.toml").to_string() }
    );
}

#[test]
fn select_target_root_returns_no_cargo_toml_when_no_manifest() {
    let outer = Utf8Path::new("/repo").to_path_buf();
    let observations = vec![observed_dir(outer.as_std_path(), ManifestKind::Missing)];
    let manifests: Vec<titania_core::ManifestObservation> = vec![];

    let err = select_target_root(&observations, &manifests).unwrap_err();
    assert_eq!(err, TargetProjectError::NoCargoToml);
}

#[test]
fn classify_manifest_detects_workspace_and_package() {
    assert_eq!(
        titania_core::classify_manifest("[workspace]\nmembers = [\"a\"]\n"),
        titania_core::ManifestStatus::Workspace
    );
    assert_eq!(
        titania_core::classify_manifest(cargo_manifest("crate").as_str()),
        titania_core::ManifestStatus::Package
    );
    assert_eq!(
        titania_core::classify_manifest("[workspace.metadata]\nkind = \"x\"\n"),
        titania_core::ManifestStatus::Other
    );
    assert_eq!(
        titania_core::classify_manifest("[workspace\nmembers = [\"a\"]"),
        titania_core::ManifestStatus::Malformed
    );
}

// Reference the unused `cargo_manifest` helper so unused-warnings stay quiet
// when individual tests below are added.
#[allow(dead_code)]
fn _force_use_cargo_manifest() -> String {
    cargo_manifest("noop")
}

#[allow(dead_code)]
fn _force_use_pathbuf() -> PathBuf {
    PathBuf::from("/tmp/noop")
}
