//! Action-shell adapter for target-project discovery.
//!
//! Walks ancestor directories from a starting path, reads each `Cargo.toml`
//! it can find, classifies the manifest via [`titania_core::classify_manifest`],
//! and feeds the resulting observations into the pure core selector. The
//! shell owns every filesystem read; the core only reasons over the typed
//! observations the shell hands it.
//!
//! This is the single allowed filesystem boundary for target discovery.
//! The rest of the workspace treats the produced [`TargetProject`] as an
//! already-validated value and never re-reads its manifest.

use std::{env, io, path::Path};

use camino::{Utf8Path, Utf8PathBuf};
use thiserror::Error;
use titania_core::{
    ManifestKind, ManifestObservation, TargetObservation, TargetProject, TargetProjectError,
    classify_manifest, select_target_observation,
};

type TargetResult<T> = Result<T, TargetProjectError>;

#[derive(Debug)]
struct ObservationSet {
    targets: Vec<TargetObservation>,
    manifests: Vec<ManifestObservation>,
}

impl ObservationSet {
    fn with_capacity(capacity: usize) -> Self {
        Self { targets: Vec::with_capacity(capacity), manifests: Vec::with_capacity(capacity) }
    }

    fn push(&mut self, ancestor: AncestorObservation) {
        self.targets.push(ancestor.target);
        self.manifests.extend(ancestor.manifest);
    }
}

#[derive(Debug)]
struct AncestorObservation {
    target: TargetObservation,
    manifest: Option<ManifestObservation>,
}

#[derive(Debug)]
struct RootClassification {
    is_directory: bool,
    manifest: ManifestKind,
}

/// Errors produced while resolving the target project from the process CWD.
#[derive(Debug, Error)]
pub enum CurrentTargetError {
    /// The process current working directory could not be read.
    #[error("cannot read current directory")]
    CurrentDir(#[source] io::Error),
    /// No valid Cargo target project could be resolved from the CWD.
    #[error(transparent)]
    Target(#[from] TargetProjectError),
}

/// Construct a [`TargetProject`] from an arbitrary filesystem path.
///
/// Walks ancestors from the given path, reads manifests, and selects the
/// nearest workspace root (or single-package root). Every filesystem read
/// happens here; the pure core only reasons over the resulting
/// observations.
///
/// # Errors
/// Returns a [`TargetProjectError`] when the path cannot be resolved to
/// a valid Cargo target project.
pub fn discover_target(cwd: &Path) -> Result<TargetProject, TargetProjectError> {
    let utf8_root = Utf8Path::from_path(cwd).ok_or(TargetProjectError::NotUtf8)?;
    if !utf8_root.is_absolute() {
        return Err(TargetProjectError::NonAbsolute(utf8_root.to_string()));
    }

    let ancestors: Vec<Utf8PathBuf> = utf8_root.ancestors().map(Utf8Path::to_path_buf).collect();

    let observations = collect_observations(&ancestors)?;
    let selected = select_target_observation(&observations.targets, &observations.manifests)?;
    TargetProject::try_from_observation(selected)
}

/// Discover the target Rust project from the current working directory.
///
/// Lanes are launched from the project they should judge; this helper is
/// the single adapter that turns the ambient CWD into the typed
/// [`TargetProject`] value used by subprocess code.
///
/// # Errors
/// Returns [`CurrentTargetError::CurrentDir`] when CWD cannot be read and
/// [`CurrentTargetError::Target`] when no valid Cargo target project can
/// be discovered from that directory.
pub fn current_target_project() -> Result<TargetProject, CurrentTargetError> {
    let cwd = env::current_dir().map_err(CurrentTargetError::CurrentDir)?;
    discover_target(&cwd).map_err(CurrentTargetError::from)
}

/// Construct a [`TargetProject`] from a filesystem path, performing all
/// filesystem validation in this shell layer.
///
/// The pure core cannot read the filesystem; this function is the canonical
/// shell-side constructor for cases where the caller already knows the
/// candidate root (not the discovery walker).
///
/// # Errors
/// Returns the same [`TargetProjectError`] variants the previous
/// `TargetProject::try_from_path` produced.
pub fn try_from_path(path: &Path) -> Result<TargetProject, TargetProjectError> {
    let utf8_path = Utf8Path::from_path(path).ok_or(TargetProjectError::NotUtf8)?;
    if utf8_path.as_str().is_empty() {
        return Err(TargetProjectError::Empty);
    }
    if !utf8_path.is_absolute() {
        return Err(TargetProjectError::NonAbsolute(utf8_path.to_string()));
    }
    let observation = observe_root(utf8_path)?;
    TargetProject::try_from_observation(&observation)
}

/// Walk the ancestor chain, returning the per-ancestor metadata observations
/// and the manifest-text observations used by the pure selector.
///
/// # Errors
/// Returns [`TargetProjectError`] when any ancestor metadata or manifest read
/// fails.
fn collect_observations(ancestors: &[Utf8PathBuf]) -> TargetResult<ObservationSet> {
    ancestors
        .iter()
        .map(Utf8PathBuf::as_path)
        .try_fold(ObservationSet::with_capacity(ancestors.len()), append_observation)
}

/// Append one observed ancestor to the accumulated discovery observations.
///
/// # Errors
/// Returns [`TargetProjectError`] when reading the ancestor metadata or
/// manifest fails.
fn append_observation(
    mut observations: ObservationSet,
    root: &Utf8Path,
) -> TargetResult<ObservationSet> {
    observations.push(observe_ancestor(root)?);
    Ok(observations)
}

/// Read one ancestor's directory metadata and (if present) its manifest text.
///
/// # Errors
/// Returns [`TargetProjectError`] when root metadata or manifest reading fails.
fn observe_ancestor(root: &Utf8Path) -> TargetResult<AncestorObservation> {
    let manifest_path = root.join("Cargo.toml");
    let classification = classify_root(root, &manifest_path)?;
    let manifest = manifest_observation(root, &manifest_path, classification.manifest)?;
    Ok(AncestorObservation {
        target: TargetObservation::new(
            root.to_path_buf(),
            classification.is_directory,
            classification.manifest,
        ),
        manifest,
    })
}

/// Read the manifest text for file manifests and skip absent or non-file manifests.
///
/// # Errors
/// Returns [`TargetProjectError::Io`] when a present manifest file cannot be
/// read, or [`TargetProjectError::NoCargoToml`] if it disappears before read.
fn manifest_observation(
    root: &Utf8Path,
    manifest_path: &Utf8Path,
    manifest: ManifestKind,
) -> TargetResult<Option<ManifestObservation>> {
    match manifest {
        ManifestKind::File => read_manifest(root, manifest_path).map(Some),
        ManifestKind::Directory | ManifestKind::Missing => Ok(None),
    }
}

/// Read the directory and manifest metadata for a single root.
///
/// # Errors
/// Returns [`TargetProjectError::NotADirectory`] when `root` exists but is not a
/// directory, or [`TargetProjectError::Io`] when root or manifest metadata fails.
fn classify_root(root: &Utf8Path, manifest_path: &Utf8Path) -> TargetResult<RootClassification> {
    match metadata_at(root)? {
        None => Ok(RootClassification { is_directory: false, manifest: ManifestKind::Missing }),
        Some(metadata) if metadata.is_dir() => Ok(RootClassification {
            is_directory: true,
            manifest: manifest_kind_at(manifest_path)?,
        }),
        Some(_) => Err(TargetProjectError::NotADirectory),
    }
}

/// Read filesystem metadata, mapping absent paths to `None`.
///
/// # Errors
/// Returns [`TargetProjectError::Io`] when metadata fails for any reason other
/// than [`io::ErrorKind::NotFound`].
fn metadata_at(path: &Utf8Path) -> TargetResult<Option<std::fs::Metadata>> {
    match std::fs::metadata(path.as_std_path()) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(TargetProjectError::Io { path: path.to_string(), kind: error.kind() }),
    }
}

/// Classify the candidate manifest path.
///
/// # Errors
/// Returns [`TargetProjectError::Io`] when manifest metadata cannot be read.
fn manifest_kind_at(manifest_path: &Utf8Path) -> TargetResult<ManifestKind> {
    metadata_at(manifest_path).map(manifest_kind_from_metadata)
}

fn manifest_kind_from_metadata(metadata: Option<std::fs::Metadata>) -> ManifestKind {
    match metadata {
        Some(file) if file.is_file() => ManifestKind::File,
        Some(_) => ManifestKind::Directory,
        None => ManifestKind::Missing,
    }
}

/// Read and classify one manifest's text. Returns
/// `Malformed` if the file's contents do not parse as TOML.
///
/// # Errors
/// Returns [`TargetProjectError::Io`] when the manifest cannot be read, or
/// [`TargetProjectError::NoCargoToml`] when it disappears before read.
fn read_manifest(root: &Utf8Path, manifest_path: &Utf8Path) -> TargetResult<ManifestObservation> {
    let text = std::fs::read_to_string(manifest_path.as_std_path())
        .map_err(|error| manifest_read_error(manifest_path, error.kind()))?;
    Ok(ManifestObservation {
        root: root.to_path_buf(),
        manifest_path: manifest_path.to_path_buf(),
        status: classify_manifest(&text),
    })
}

fn manifest_read_error(manifest_path: &Utf8Path, kind: io::ErrorKind) -> TargetProjectError {
    match kind {
        io::ErrorKind::NotFound => TargetProjectError::NoCargoToml,
        kind => TargetProjectError::Io { path: manifest_path.to_string(), kind },
    }
}

/// Observe a single candidate root (no ancestor walk) for the
/// `try_from_path` shell constructor.
///
/// # Errors
/// Returns [`TargetProjectError`] when the root path or manifest metadata does
/// not satisfy the target-project filesystem contract.
fn observe_root(root: &Utf8Path) -> TargetResult<TargetObservation> {
    let manifest_path = root.join("Cargo.toml");
    let classification = classify_root(root, &manifest_path)?;
    Ok(TargetObservation::new(
        root.to_path_buf(),
        classification.is_directory,
        classification.manifest,
    ))
}

/// Convenience wrapper matching the historical
/// `titania_lanes::target_project_from_path` signature.
///
/// # Errors
/// Returns [`TargetProjectError`] when `cwd` cannot be resolved to a valid
/// Cargo target project.
pub fn target_project_from_path(cwd: &Path) -> Result<TargetProject, TargetProjectError> {
    discover_target(cwd)
}
