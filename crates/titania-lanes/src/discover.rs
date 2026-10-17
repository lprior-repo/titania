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

#[derive(Debug, Error)]
pub enum CurrentTargetError {
    #[error("cannot read current directory")]
    CurrentDir(#[source] io::Error),
    #[error(transparent)]
    Target(#[from] TargetProjectError),
}

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

pub fn current_target_project() -> Result<TargetProject, CurrentTargetError> {
    let cwd = env::current_dir().map_err(CurrentTargetError::CurrentDir)?;
    discover_target(&cwd).map_err(CurrentTargetError::from)
}

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

fn collect_observations(ancestors: &[Utf8PathBuf]) -> TargetResult<ObservationSet> {
    ancestors
        .iter()
        .map(Utf8PathBuf::as_path)
        .try_fold(ObservationSet::with_capacity(ancestors.len()), append_observation)
}

fn append_observation(
    mut observations: ObservationSet,
    root: &Utf8Path,
) -> TargetResult<ObservationSet> {
    observations.push(observe_ancestor(root)?);
    Ok(observations)
}

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

fn metadata_at(path: &Utf8Path) -> TargetResult<Option<std::fs::Metadata>> {
    match std::fs::metadata(path.as_std_path()) {
        Ok(metadata) => Ok(Some(metadata)),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(TargetProjectError::Io { path: path.to_string(), kind: error.kind() }),
    }
}

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

fn observe_root(root: &Utf8Path) -> TargetResult<TargetObservation> {
    let manifest_path = root.join("Cargo.toml");
    let classification = classify_root(root, &manifest_path)?;
    Ok(TargetObservation::new(
        root.to_path_buf(),
        classification.is_directory,
        classification.manifest,
    ))
}

pub fn target_project_from_path(cwd: &Path) -> Result<TargetProject, TargetProjectError> {
    discover_target(cwd)
}
