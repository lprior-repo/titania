use std::path::{Path, PathBuf};

use titania_core::{LaneFailure, TargetProject};
use toml_edit::{Item, Value};

use crate::{LaneReport, RuleId};

const LIB_TITANIA_DYLINT: &str = "libtitania_dylint";
const RULE_DYLINT_INFRA: &str = "DYLINT_INFRA_FAILURE";
const TITANIA_DYLINT_LIB_ENV: &str = "TITANIA_DYLINT_LIB";
const TITANIA_DYLINT_PACKAGE: &str = "titania-dylint";
const TITANIA_DYLINT_CRATE: &str = "titania_dylint";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DylintLibrarySource {
    Env,
    Sibling,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DylintLoad {
    Metadata,
    LibraryPath { source: DylintLibrarySource, path: PathBuf },
}

impl DylintLoad {
    #[must_use]
    pub fn lib_path(&self) -> Option<&Path> {
        match self {
            Self::Metadata => None,
            Self::LibraryPath { path, .. } => Some(path.as_path()),
        }
    }
}

fn cargo_dylint_available(target: &TargetProject) -> bool {
    crate::command::CommandIn::new(target, "cargo")
        .and_then(|mut cmd| cmd.inherit_env().args(&["dylint", "--help"]).run_capture_raw())
        .is_ok_and(|output| output.success())
}
#[derive(Debug)]
pub enum DylintProbe {
    Ready(DylintLoad),
    Infra(LaneFailure, LaneReport),
}

impl DylintProbe {
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    #[must_use]
    pub const fn load(&self) -> Option<&DylintLoad> {
        match self {
            Self::Ready(load) => Some(load),
            Self::Infra(_, _) => None,
        }
    }

    #[must_use]
    pub const fn failure(&self) -> Option<&LaneFailure> {
        match self {
            Self::Ready(_) => None,
            Self::Infra(failure, _) => Some(failure),
        }
    }
}

#[must_use]
pub fn probe_dylint_toolchain(target: &TargetProject) -> DylintProbe {
    if !cargo_dylint_available(target) {
        return unavailable_probe("cargo-dylint", String::from("subcommand unavailable"));
    }

    match probe_libtitania_dylint(target) {
        LibraryProbe::Compatible(load) => DylintProbe::Ready(load),
        LibraryProbe::Absent => unavailable_probe(
            LIB_TITANIA_DYLINT,
            format!(
                "{LIB_TITANIA_DYLINT} unavailable: no plugin library found by env/metadata/sibling lookup"
            ),
        ),
        LibraryProbe::Incompatible { path, reason } => unavailable_probe(
            LIB_TITANIA_DYLINT,
            format!("{LIB_TITANIA_DYLINT} ABI mismatch at {}: {reason}", path.display()),
        ),
    }
}

#[derive(Debug)]
enum LibraryProbe {
    Compatible(DylintLoad),
    Absent,
    Incompatible { path: std::path::PathBuf, reason: String },
}

fn probe_libtitania_dylint(target: &TargetProject) -> LibraryProbe {
    resolve_libtitania_dylint(
        env_library_path(),
        workspace_metadata_dylint_configured(target.as_std_path()),
        sibling_library_path(),
    )
}

fn resolve_libtitania_dylint(
    env_path: Option<PathBuf>,
    metadata_names_titania: bool,
    sibling_path: Option<PathBuf>,
) -> LibraryProbe {
    if let Some(path) = env_path {
        return probe_library_path(DylintLibrarySource::Env, path);
    }
    if metadata_names_titania {
        return LibraryProbe::Compatible(DylintLoad::Metadata);
    }
    sibling_path
        .map_or(LibraryProbe::Absent, |path| probe_library_path(DylintLibrarySource::Sibling, path))
}

fn env_library_path() -> Option<PathBuf> {
    std::env::var_os(TITANIA_DYLINT_LIB_ENV).map(PathBuf::from)
}

fn probe_library_path(source: DylintLibrarySource, path: PathBuf) -> LibraryProbe {
    if !path.is_file() {
        return LibraryProbe::Incompatible {
            path,
            reason: String::from("configured library path is not a file"),
        };
    }
    if path.to_str().is_none() {
        return LibraryProbe::Incompatible {
            path,
            reason: String::from("configured library path is not valid UTF-8"),
        };
    }
    if let Some(reason) = abi_check(&path) {
        return LibraryProbe::Incompatible { path, reason };
    }
    LibraryProbe::Compatible(DylintLoad::LibraryPath { source, path })
}

fn workspace_metadata_dylint_configured(root: &Path) -> bool {
    match std::fs::read_to_string(root.join("Cargo.toml")) {
        Ok(content) => cargo_manifest_names_titania_dylint(&content),
        Err(_error) => false,
    }
}

fn cargo_manifest_names_titania_dylint(content: &str) -> bool {
    content.parse::<toml_edit::DocumentMut>().is_ok_and(|document| metadata_libraries(&document))
}

fn metadata_libraries(document: &toml_edit::DocumentMut) -> bool {
    document
        .get("workspace")
        .and_then(Item::as_table)
        .and_then(|workspace| workspace.get("metadata"))
        .and_then(Item::as_table)
        .and_then(|metadata| metadata.get("dylint"))
        .and_then(Item::as_table)
        .and_then(|dylint| dylint.get("libraries"))
        .is_some_and(metadata_item_names_titania)
}

fn metadata_item_names_titania(item: &Item) -> bool {
    if let Some(value) = item.as_value() {
        return metadata_value_names_titania(value);
    }
    if let Some(table) = item.as_table() {
        return metadata_table_names_titania(table);
    }
    item.as_array_of_tables().is_some_and(|tables| tables.iter().any(metadata_table_names_titania))
}

fn metadata_table_names_titania(table: &toml_edit::Table) -> bool {
    table.iter().any(|(_key, item)| item.as_value().is_some_and(metadata_value_names_titania))
}

fn metadata_value_names_titania(value: &Value) -> bool {
    if let Some(text) = value.as_str() {
        return metadata_string_names_titania(text);
    }
    if let Some(array) = value.as_array() {
        return array.iter().any(metadata_array_value_names_titania);
    }
    value.as_inline_table().is_some_and(metadata_inline_table_names_titania)
}

fn metadata_array_value_names_titania(value: &Value) -> bool {
    if let Some(text) = value.as_str() {
        return metadata_string_names_titania(text);
    }
    value.as_inline_table().is_some_and(metadata_inline_table_names_titania)
}

fn metadata_inline_table_names_titania(table: &toml_edit::InlineTable) -> bool {
    table.iter().any(|(_key, value)| metadata_value_names_titania(value))
}

fn metadata_string_names_titania(value: &str) -> bool {
    value.split(['/', '\\']).any(metadata_segment_names_titania)
}

fn metadata_segment_names_titania(segment: &str) -> bool {
    let candidate = strip_dynamic_library_extension(segment);
    candidate == TITANIA_DYLINT_PACKAGE
        || candidate == TITANIA_DYLINT_CRATE
        || candidate == LIB_TITANIA_DYLINT
}

fn strip_dynamic_library_extension(segment: &str) -> &str {
    if let Some(stripped) = segment.strip_suffix(".so") {
        return stripped;
    }
    if let Some(stripped) = segment.strip_suffix(".dylib") {
        return stripped;
    }
    if let Some(stripped) = segment.strip_suffix(".dll") {
        return stripped;
    }
    segment
}

fn sibling_library_path() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let dir = exe.parent()?;
    dylint_library_names().into_iter().map(|name| dir.join(name)).find(|path| path.is_file())
}

fn dylint_library_names() -> [String; 3] {
    [
        format!("{LIB_TITANIA_DYLINT}.so"),
        format!("{LIB_TITANIA_DYLINT}.dylib"),
        String::from("titania_dylint.dll"),
    ]
}

fn abi_check(path: &Path) -> Option<String> {
    let bytes = match std::fs::read(path) {
        Ok(bytes) => bytes,
        Err(e) => return Some(format!("cannot read library at {}: {e}", path.display())),
    };
    let Some(head) = bytes.first_chunk::<4>().copied() else {
        return Some(format!(
            "library at {} is too short ({} bytes) to be a dynamic library",
            path.display(),
            bytes.len()
        ));
    };
    if !is_dynamic_library_magic(head) {
        return Some(format!(
            "library at {} is not a dynamic library (missing ELF/Mach-O/PE magic; got bytes {:02x?})",
            path.display(),
            head
        ));
    }
    None
}

const fn is_dynamic_library_magic(head: [u8; 4]) -> bool {
    let [b0, b1, b2, b3] = head;
    if b0 == 0x7f && b1 == b'E' && b2 == b'L' && b3 == b'F' {
        return true;
    }
    if b0 == 0xfe && b1 == 0xed && b2 == 0xfa && (b3 == 0xce || b3 == 0xcf) {
        return true;
    }
    if b0 == 0xce && b1 == 0xfa && b2 == 0xed && b3 == 0xfe {
        return true;
    }
    if b0 == 0xcf && b1 == 0xfa && b2 == 0xed && b3 == 0xfe {
        return true;
    }
    if b0 == b'M' && b1 == b'Z' {
        return true;
    }
    false
}

#[must_use]
pub(crate) fn path_needs_staging(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()).is_none_or(|name| !name.contains('@'))
}

fn active_toolchain_string(target: &TargetProject) -> Option<String> {
    let output = crate::command::CommandIn::new(target, "rustup")
        .and_then(|mut cmd| cmd.inherit_env().args(&["show", "active-toolchain"]).run_capture_raw())
        .ok()?;
    if !output.success() {
        return None;
    }
    let stdout = output.stdout_str().ok()?;
    stdout.split_ascii_whitespace().next().map(str::to_owned)
}

fn toolchain_library_filename(toolchain: &str) -> String {
    format!(
        "{}{TITANIA_DYLINT_CRATE}@{toolchain}{}",
        std::env::consts::DLL_PREFIX,
        std::env::consts::DLL_SUFFIX,
    )
}

fn create_staging_dir() -> Option<PathBuf> {
    let base = std::env::temp_dir();
    let pid = std::process::id();
    let primary = base.join(format!("titania-dylint-{pid}"));
    if std::fs::create_dir(&primary).is_ok() {
        return Some(primary);
    }
    (1u32..=100).find_map(|suffix| {
        let path = base.join(format!("titania-dylint-{pid}-{suffix}"));
        std::fs::create_dir(&path).is_ok().then_some(path)
    })
}

pub(crate) struct DylintLibStaging {
    dir: PathBuf,
    lib_path: PathBuf,
}

impl DylintLibStaging {
    #[must_use]
    pub(crate) fn lib_path(&self) -> &Path {
        &self.lib_path
    }

    pub(crate) fn stage(source: &Path, target: &TargetProject) -> Result<Self, LaneFailure> {
        let toolchain = active_toolchain_string(target).ok_or_else(|| LaneFailure::Infra {
            tool: String::from(LIB_TITANIA_DYLINT),
            reason: String::from(
                "cannot determine active rust toolchain (rustup show active-toolchain failed) \
                 for dylint library staging",
            ),
        })?;
        let dir = create_staging_dir().ok_or_else(|| LaneFailure::Infra {
            tool: String::from(LIB_TITANIA_DYLINT),
            reason: String::from("cannot create temp directory for dylint library staging"),
        })?;
        let lib_path = dir.join(toolchain_library_filename(&toolchain));
        let _copied = std::fs::copy(source, &lib_path).map_err(|err| LaneFailure::Infra {
            tool: String::from(LIB_TITANIA_DYLINT),
            reason: format!("cannot stage dylint library to {}: {err}", lib_path.display()),
        })?;
        Ok(Self { dir, lib_path })
    }
}

impl Drop for DylintLibStaging {
    fn drop(&mut self) {
        let _cleanup_result = std::fs::remove_dir_all(&self.dir);
    }
}

fn unavailable_probe(tool: &str, reason: String) -> DylintProbe {
    let failure = LaneFailure::Infra { tool: tool.to_owned(), reason };

    match infra_report(tool) {
        Ok(report) => DylintProbe::Infra(failure, report),
        Err(report_failure) => DylintProbe::Infra(report_failure, LaneReport::new()),
    }
}

fn infra_report(tool: &str) -> Result<LaneReport, LaneFailure> {
    let mut report = LaneReport::new();
    let rule = RuleId::new(RULE_DYLINT_INFRA).map_err(|err| LaneFailure::Infra {
        tool: String::from("titania-dylint"),
        reason: format!("invalid Dylint infra rule id {RULE_DYLINT_INFRA}: {err}"),
    })?;

    report.push(crate::Finding::new(rule, tool, 0, format!("{tool} is unavailable")));
    Ok(report)
}

#[cfg(test)]
mod tests {
    use std::{fs, path::Path};

    use super::{
        DylintLibrarySource, DylintLoad, LibraryProbe, cargo_manifest_names_titania_dylint,
        resolve_libtitania_dylint,
    };

    #[test]
    fn metadata_must_name_titania_library() {
        let unrelated = r#"
[workspace.metadata.dylint]
libraries = [{ path = "crates/unrelated-lint" }]
"#;
        let titania = r#"
[workspace.metadata.dylint]
libraries = [{ path = "crates/titania-dylint" }]
"#;

        assert!(!cargo_manifest_names_titania_dylint(unrelated));
        assert!(cargo_manifest_names_titania_dylint(titania));
    }

    #[test]
    fn env_library_path_wins_over_metadata_and_sibling() {
        let temp = tempfile::tempdir().expect("tempdir must be created");
        let env_path = write_fake_library(temp.path(), "env_libtitania_dylint.so");
        let sibling_path = write_fake_library(temp.path(), "sibling_libtitania_dylint.so");

        let probe = resolve_libtitania_dylint(Some(env_path.clone()), true, Some(sibling_path));

        let LibraryProbe::Compatible(DylintLoad::LibraryPath { source, path }) = probe else {
            panic!("env path must resolve to a concrete library load");
        };
        assert_eq!(source, DylintLibrarySource::Env);
        assert_eq!(path, env_path);
    }

    #[test]
    fn metadata_wins_over_sibling_when_it_names_titania() {
        let temp = tempfile::tempdir().expect("tempdir must be created");
        let sibling_path = write_fake_library(temp.path(), "sibling_libtitania_dylint.so");

        let probe = resolve_libtitania_dylint(None, true, Some(sibling_path));

        let LibraryProbe::Compatible(DylintLoad::Metadata) = probe else {
            panic!("Titania metadata must resolve to metadata load mode");
        };
    }

    fn write_fake_library(dir: &Path, name: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        fs::write(&path, b"\x7fELFfake-dylint-library").expect("fake library file must be written");
        path
    }
}
