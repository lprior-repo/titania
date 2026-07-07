//! Dylint lane loader — checks tool availability and reports typed infra failures.
//!
//! This module owns the Dylint lane's pre-flight checks:
//! 1. Is `cargo-dylint` available (via `cargo dylint --help`)? If not,
//!    return [`LaneFailure::Infra`] with `tool = "cargo-dylint"`.
//! 2. Is the `libtitania_dylint` cdylib available and ABI-compatible? If not,
//!    return [`LaneFailure::Infra`] with `tool = "libtitania_dylint"`.
//!
//! No lint logic lives here — only the load / wiring contract.

use std::path::{Path, PathBuf};

use titania_core::{LaneFailure, TargetProject};
use toml_edit::{Item, Value};

use crate::{LaneReport, RuleId};

const LIB_TITANIA_DYLINT: &str = "libtitania_dylint";
const RULE_DYLINT_INFRA: &str = "DYLINT_INFRA_FAILURE";
const TITANIA_DYLINT_LIB_ENV: &str = "TITANIA_DYLINT_LIB";
const TITANIA_DYLINT_PACKAGE: &str = "titania-dylint";
const TITANIA_DYLINT_CRATE: &str = "titania_dylint";

/// Source that supplied a concrete `libtitania_dylint` dynamic library path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DylintLibrarySource {
    /// The `TITANIA_DYLINT_LIB` environment variable supplied the path.
    Env,
    /// A sibling library beside the running `titania-check` binary supplied the path.
    Sibling,
}

/// How `cargo dylint` should load Titania's Dylint library.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DylintLoad {
    /// Use cargo-dylint workspace metadata (`[workspace.metadata.dylint]`).
    Metadata,
    /// Pass a resolved dynamic library file with `--lib-path <path>`.
    LibraryPath {
        /// Lookup source that provided the path.
        source: DylintLibrarySource,
        /// Resolved path to `libtitania_dylint`.
        path: PathBuf,
    },
}

impl DylintLoad {
    /// Return the concrete dynamic library path when this load uses `--lib-path`.
    #[must_use]
    pub fn lib_path(&self) -> Option<&Path> {
        match self {
            Self::Metadata => None,
            Self::LibraryPath { path, .. } => Some(path.as_path()),
        }
    }
}

/// Probe `cargo dylint --help` availability.
///
/// Uses `cargo dylint --help` (the subcommand form avoids
/// the `cargo-dylint` shim). Checks `.success()`.
fn cargo_dylint_available(target: &TargetProject) -> bool {
    crate::command::CommandIn::new(target, "cargo")
        .and_then(|mut cmd| cmd.inherit_env().args(&["dylint", "--help"]).run_capture_raw())
        .is_ok_and(|output| output.success())
}
/// Result of the Dylint lane pre-flight checks.
#[derive(Debug)]
pub enum DylintProbe {
    /// Both tools are available; the lane can proceed with this load strategy.
    Ready(DylintLoad),
    /// Infrastructure probe failed with typed details.
    Infra(LaneFailure, LaneReport),
}

impl DylintProbe {
    /// Whether the probe indicates the lane can proceed.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready(_))
    }

    /// Borrow the resolved Dylint load strategy when the probe succeeded.
    #[must_use]
    pub const fn load(&self) -> Option<&DylintLoad> {
        match self {
            Self::Ready(load) => Some(load),
            Self::Infra(_, _) => None,
        }
    }

    /// Borrow failure details when the probe did not succeed.
    #[must_use]
    pub const fn failure(&self) -> Option<&LaneFailure> {
        match self {
            Self::Ready(_) => None,
            Self::Infra(failure, _) => Some(failure),
        }
    }
}

/// Probe the Dylint toolchain for availability.
///
/// Returns [`DylintProbe::Ready`] when both `cargo-dylint` and
/// `libtitania_dylint` are available. Returns [`DylintProbe::Infra`]
/// on the first infrastructure failure encountered.
#[must_use]
pub fn probe_dylint_toolchain(target: &TargetProject) -> DylintProbe {
    // Check 1: is cargo-dylint available (via `cargo dylint --help`)?
    if !cargo_dylint_available(target) {
        return unavailable_probe("cargo-dylint", String::from("subcommand unavailable"));
    }

    // Check 2: is libtitania_dylint available and ABI-compatible?
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

/// Result of probing the `libtitania_dylint` cdylib (bead tn-gkpv).
#[derive(Debug)]
enum LibraryProbe {
    /// Library exists and looks loadable.
    Compatible(DylintLoad),
    /// Library was not found in any target dir candidate.
    Absent,
    /// Library exists but failed ABI / format / magic checks.
    Incompatible {
        /// Resolved path to the failing library file.
        path: std::path::PathBuf,
        /// Short reason explaining the failure.
        reason: String,
    },
}

/// Check whether the `libtitania_dylint` cdylib is available and ABI-compatible.
///
/// Bead tn-gkpv: a missing OR incompatible library must surface a typed
/// [`LaneFailure::Infra`] with a distinct reason — never silently pass.
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

/// Minimal ABI sanity check on the resolved library path.
///
/// Today we verify the file is a loadable dynamic library by checking the
/// platform magic (ELF on Linux, Mach-O on macOS, PE on Windows). When the
/// dylint loader evolves to require an exported `dylint_version` symbol, this
/// is the place to extend it. Returns `None` on success, or `Some(reason)` with
/// a short human-readable reason suitable for [`LaneFailure::Infra.reason`].
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

/// Return `true` when the 4-byte head of a candidate library file matches the
/// platform magic for an ELF, Mach-O, or PE (DOS MZ) dynamic library.
const fn is_dynamic_library_magic(head: [u8; 4]) -> bool {
    let [b0, b1, b2, b3] = head;
    // ELF: 0x7F 'E' 'L' 'F'
    if b0 == 0x7f && b1 == b'E' && b2 == b'L' && b3 == b'F' {
        return true;
    }
    // Mach-O variants
    if b0 == 0xfe && b1 == 0xed && b2 == 0xfa && (b3 == 0xce || b3 == 0xcf) {
        return true;
    }
    if b0 == 0xce && b1 == 0xfa && b2 == 0xed && b3 == 0xfe {
        return true;
    }
    if b0 == 0xcf && b1 == 0xfa && b2 == 0xed && b3 == 0xfe {
        return true;
    }
    // PE / DOS MZ ('M' 'Z')
    if b0 == b'M' && b1 == b'Z' {
        return true;
    }
    false
}

fn unavailable_probe(tool: &str, reason: String) -> DylintProbe {
    let failure = LaneFailure::Infra { tool: tool.to_owned(), reason };

    match infra_report(tool) {
        Ok(report) => DylintProbe::Infra(failure, report),
        Err(report_failure) => DylintProbe::Infra(report_failure, LaneReport::new()),
    }
}

/// Build a minimal [`LaneReport`] for an infrastructure failure.
///
/// # Errors
///
/// Returns [`LaneFailure::Infra`] if the static Dylint infra rule id violates
/// the shared rule-id format.
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
