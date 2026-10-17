use camino::Utf8PathBuf;

use crate::error::TargetProjectError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestStatus {
    Workspace,
    Package,
    Other,
    Malformed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestKind {
    File,
    Directory,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestObservation {
    pub root: Utf8PathBuf,
    pub manifest_path: Utf8PathBuf,
    pub status: ManifestStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TargetObservation {
    pub root: Utf8PathBuf,
    pub root_is_dir: bool,
    pub manifest: ManifestKind,
}

impl TargetObservation {
    #[must_use]
    pub const fn new(root: Utf8PathBuf, root_is_dir: bool, manifest: ManifestKind) -> Self {
        Self { root, root_is_dir, manifest }
    }
}

pub fn select_target_root(
    _observations: &[TargetObservation],
    manifests: &[ManifestObservation],
) -> Result<Utf8PathBuf, TargetProjectError> {
    if let Some(workspace) = manifests.iter().find(|m| m.status == ManifestStatus::Workspace) {
        return Ok(workspace.root.clone());
    }
    manifests
        .iter()
        .find(|m| matches!(m.status, ManifestStatus::Package | ManifestStatus::Malformed))
        .map_or(Err(TargetProjectError::NoCargoToml), selected_root_from_manifest)
}

pub fn select_target_observation<'a>(
    observations: &'a [TargetObservation],
    manifests: &[ManifestObservation],
) -> Result<&'a TargetObservation, TargetProjectError> {
    let selected_root = select_target_root(observations, manifests)?;
    observations.iter().find(|o| o.root == selected_root).ok_or(TargetProjectError::NoCargoToml)
}

fn selected_root_from_manifest(
    manifest: &ManifestObservation,
) -> Result<Utf8PathBuf, TargetProjectError> {
    match manifest.status {
        ManifestStatus::Workspace | ManifestStatus::Package => Ok(manifest.root.clone()),
        ManifestStatus::Malformed => {
            Err(TargetProjectError::MalformedCargoToml { path: manifest.manifest_path.to_string() })
        }
        ManifestStatus::Other => Err(TargetProjectError::NoCargoToml),
    }
}

#[must_use]
pub fn classify_manifest(toml_text: &str) -> ManifestStatus {
    match toml_text.parse::<toml_edit::DocumentMut>() {
        Ok(doc) if has_explicit_table(&doc, "workspace") => ManifestStatus::Workspace,
        Ok(doc) if has_explicit_table(&doc, "package") => ManifestStatus::Package,
        Ok(_) => ManifestStatus::Other,
        Err(_) => ManifestStatus::Malformed,
    }
}

fn has_explicit_table(doc: &toml_edit::DocumentMut, name: &str) -> bool {
    doc.get(name).and_then(toml_edit::Item::as_table).is_some_and(|table| !table.is_implicit())
}
