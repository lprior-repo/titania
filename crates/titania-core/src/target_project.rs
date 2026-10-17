use core::fmt;

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::{
    discover::{ManifestKind, TargetObservation},
    error::TargetProjectError,
};

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TargetProject(Utf8PathBuf);

impl TargetProject {
    pub fn try_from_observation(
        observation: &TargetObservation,
    ) -> Result<Self, TargetProjectError> {
        Self::try_from_validated_path(
            &observation.root,
            observation.root_is_dir,
            observation.manifest,
        )
    }

    pub fn try_from_validated_path(
        path: &Utf8Path,
        root_is_dir: bool,
        manifest: ManifestKind,
    ) -> Result<Self, TargetProjectError> {
        validate_not_empty(path)?;
        validate_absolute(path)?;
        ensure_root_directory(root_is_dir)?;
        match manifest {
            ManifestKind::File => Ok(Self(path.to_owned())),
            ManifestKind::Directory => Err(TargetProjectError::CargoTomlNotFile),
            ManifestKind::Missing => Err(TargetProjectError::NoCargoToml),
        }
    }

    pub fn try_from_path_string(path: &Utf8Path) -> Result<Self, TargetProjectError> {
        validate_not_empty(path)?;
        validate_absolute(path)?;
        Ok(Self(path.to_owned()))
    }

    #[must_use]
    pub fn as_path(&self) -> &Utf8Path {
        &self.0
    }

    #[must_use]
    pub fn manifest_path(&self) -> Utf8PathBuf {
        self.0.join("Cargo.toml")
    }

    #[must_use]
    pub fn as_std_path(&self) -> &std::path::Path {
        self.0.as_std_path()
    }
}

fn validate_not_empty(path: &Utf8Path) -> Result<(), TargetProjectError> {
    if path.as_str().is_empty() { Err(TargetProjectError::Empty) } else { Ok(()) }
}

fn validate_absolute(path: &Utf8Path) -> Result<(), TargetProjectError> {
    if path.is_absolute() { Ok(()) } else { Err(TargetProjectError::NonAbsolute(path.to_string())) }
}

fn ensure_root_directory(root_is_dir: bool) -> Result<(), TargetProjectError> {
    root_is_dir.then_some(()).ok_or(TargetProjectError::NotFound)
}

impl fmt::Display for TargetProject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.as_str())
    }
}

impl fmt::Debug for TargetProject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("TargetProject").field(&self.0.as_str()).finish()
    }
}

impl AsRef<Utf8Path> for TargetProject {
    fn as_ref(&self) -> &Utf8Path {
        &self.0
    }
}

impl Serialize for TargetProject {
    fn serialize<S: Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        ser.serialize_str(self.0.as_str())
    }
}

impl<'de> Deserialize<'de> for TargetProject {
    fn deserialize<D: Deserializer<'de>>(de: D) -> Result<Self, D::Error> {
        let s = <std::borrow::Cow<'_, str> as Deserialize>::deserialize(de)?;
        let p = Utf8PathBuf::from(s.into_owned());
        Self::try_from_path_string(p.as_path()).map_err(serde::de::Error::custom)
    }
}
