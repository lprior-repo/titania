//! Doctor tool/version diagnostic domain model.
//!
//! This module owns the scope-to-tool matrix and host probing needed by
//! `titania-check doctor`. It does not render CLI output.

use std::path::{Path, PathBuf};

use crate::{OutputComponent, OutputError};
use titania_core::GateScope;

/// Outcome of a doctor scan.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DoctorStatus {
    /// All required tools are installed.
    Ok,
    /// One or more required tools are missing.
    MissingRequiredTools,
}

impl DoctorStatus {
    /// Stable external status string.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "OK",
            Self::MissingRequiredTools => "MissingRequiredTools",
        }
    }
}

/// A single tool row in the doctor report.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ToolRow {
    /// Canonical display name.
    pub name: &'static str,
    /// Whether the scope requires this tool to be usable.
    pub required: bool,
    /// Whether the tool is available to this process.
    pub installed: bool,
    /// Detected version or status detail, when available.
    pub version: Option<String>,
    /// Detected filesystem path, when available.
    pub path: Option<PathBuf>,
}

impl ToolRow {
    /// Create an embedded capability row.
    #[must_use]
    pub const fn embedded(name: &'static str) -> Self {
        Self { name, required: true, installed: true, version: None, path: None }
    }

    /// Create a probed external-tool row.
    #[must_use]
    pub const fn external(
        name: &'static str,
        required: bool,
        installed: bool,
        version: Option<String>,
        path: Option<PathBuf>,
    ) -> Self {
        Self { name, required, installed, version, path }
    }
}

/// Complete doctor report for a single gate scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctorReport {
    /// Scope the report covers.
    pub scope: GateScope,
    /// Ordered tool rows.
    pub tools: Vec<ToolRow>,
    /// Required tool names that are unavailable.
    pub missing_required: Vec<String>,
    /// Aggregate doctor status.
    pub status: DoctorStatus,
}

impl DoctorReport {
    /// Build a report from tool rows.
    #[must_use]
    pub fn new(scope: GateScope, tools: Vec<ToolRow>) -> Self {
        let missing_required = tools
            .iter()
            .filter(|row| row.required && !row.installed)
            .map(|row| row.name.to_owned())
            .collect::<Vec<_>>();
        let status = status_from_missing(missing_required.is_empty());

        Self { scope, tools, missing_required, status }
    }
}

const fn status_from_missing(no_missing_required: bool) -> DoctorStatus {
    if no_missing_required { DoctorStatus::Ok } else { DoctorStatus::MissingRequiredTools }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ToolKind {
    Binary,
    Dylint,
    Embedded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ToolConfig {
    name: &'static str,
    required: bool,
    kind: ToolKind,
}

impl ToolConfig {
    const fn binary(name: &'static str, required: bool) -> Self {
        Self { name, required, kind: ToolKind::Binary }
    }

    const fn dylint(required: bool) -> Self {
        Self { name: "cargo-dylint", required, kind: ToolKind::Dylint }
    }

    const fn embedded(name: &'static str) -> Self {
        Self { name, required: true, kind: ToolKind::Embedded }
    }
}

fn tool_configs_for_scope(scope: GateScope) -> Vec<ToolConfig> {
    let deny_required = matches!(scope, GateScope::Prepush | GateScope::Release);
    vec![
        ToolConfig::binary("cargo", true),
        ToolConfig::binary("rustfmt", true),
        ToolConfig::binary("clippy-driver", true),
        ToolConfig::binary("rg", true),
        ToolConfig::embedded("ast-grep"),
        ToolConfig::dylint(true),
        ToolConfig::binary("cargo-deny", deny_required),
        ToolConfig::binary("sccache", false),
    ]
}

fn probe_config(config: ToolConfig) -> Vec<ToolRow> {
    match config.kind {
        ToolKind::Binary => vec![probe_binary(config.name, config.required)],
        ToolKind::Dylint => probe_dylint(config.required),
        ToolKind::Embedded => vec![ToolRow::embedded(config.name)],
    }
}

fn executable_names(name: &str) -> Vec<String> {
    if cfg!(windows) { vec![format!("{name}.exe"), name.to_owned()] } else { vec![name.to_owned()] }
}

#[cfg(unix)]
fn is_runnable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;

    path.is_file()
        && path.metadata().is_ok_and(|metadata| metadata.permissions().mode() & 0o111 != 0)
}

#[cfg(windows)]
fn is_runnable_file(path: &Path) -> bool {
    path.is_file()
}

fn find_on_path(name: &str) -> Option<PathBuf> {
    let raw_path = std::env::var_os("PATH")?;
    let names = executable_names(name);
    std::env::split_paths(&raw_path).find_map(|directory| {
        names.iter().map(|candidate| directory.join(candidate)).find(|path| is_runnable_file(path))
    })
}

fn probe_binary(name: &'static str, required: bool) -> ToolRow {
    find_on_path(name).map_or_else(
        || ToolRow::external(name, required, false, None, None),
        |path| ToolRow::external(name, required, true, probe_version(&path), Some(path)),
    )
}

fn probe_version(bin_path: &Path) -> Option<String> {
    ["--version", "-V", "version"].iter().find_map(|arg| probe_version_arg(bin_path, arg))
}

fn probe_version_arg(bin_path: &Path, arg: &str) -> Option<String> {
    std::process::Command::new(bin_path)
        .arg(arg)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| extract_version(&String::from_utf8_lossy(&output.stdout)))
}

fn clean_version_token(token: &str) -> String {
    token
        .trim_matches(|ch: char| !ch.is_ascii_alphanumeric() && ch != '.' && ch != '-' && ch != '_')
        .to_owned()
}

fn version_token(line: &str) -> Option<String> {
    line.split_whitespace()
        .map(clean_version_token)
        .find(|token| token.chars().any(|ch| ch.is_ascii_digit()) && token.contains('.'))
}

fn extract_version(raw: &str) -> Option<String> {
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .find_map(version_token)
        .or_else(|| raw.lines().map(str::trim).find(|line| !line.is_empty()).map(str::to_owned))
}

fn probe_dylint(required: bool) -> Vec<ToolRow> {
    let cargo_dylint_path = find_on_path("cargo-dylint");
    let cargo_dylint_row = cargo_dylint_path.as_ref().map_or_else(
        || ToolRow::external("cargo-dylint", required, false, None, None),
        |path| {
            ToolRow::external(
                "cargo-dylint",
                required,
                true,
                probe_version(path),
                Some(path.clone()),
            )
        },
    );
    let library_row =
        probe_dylint_library(cargo_dylint_path.is_some(), cargo_dylint_path.as_deref());

    vec![cargo_dylint_row, library_row]
}

const fn dylint_library_names() -> [&'static str; 3] {
    ["libtitania_dylint.so", "libtitania_dylint.dylib", "titania_dylint.dll"]
}

fn sibling_dylint_library(cargo_dylint_path: &Path) -> Option<PathBuf> {
    cargo_dylint_path.parent().and_then(|parent| {
        dylint_library_names().iter().map(|name| parent.join(name)).find(|path| path.is_file())
    })
}

fn env_library_paths() -> Option<std::ffi::OsString> {
    ["LD_LIBRARY_PATH", "DYLD_LIBRARY_PATH"].iter().find_map(std::env::var_os)
}

fn dylint_library_in_directory(directory: &Path) -> Option<PathBuf> {
    dylint_library_names().iter().map(|name| directory.join(name)).find(|path| path.is_file())
}

fn env_dylint_library() -> Option<PathBuf> {
    env_library_paths().and_then(|paths| {
        std::env::split_paths(&paths).find_map(|directory| dylint_library_in_directory(&directory))
    })
}

fn probe_dylint_library(required: bool, cargo_dylint_path: Option<&Path>) -> ToolRow {
    let path = cargo_dylint_path.and_then(sibling_dylint_library).or_else(env_dylint_library);

    path.map_or_else(|| missing_dylint_library(required), |path| dylint_library_row(required, path))
}

fn missing_dylint_library(required: bool) -> ToolRow {
    ToolRow::external("libtitania_dylint", required, false, Some("abi:unknown".to_owned()), None)
}

fn dylint_library_row(required: bool, path: PathBuf) -> ToolRow {
    let (installed, version) =
        if abi_is_compatible(&path) { (true, "abi:verified") } else { (false, "abi:mismatch") };

    ToolRow::external(
        "libtitania_dylint",
        required,
        installed,
        Some(version.to_owned()),
        Some(path),
    )
}

/// Probe the library file for the canonical Dylint ABI markers.
///
/// Reads the file directly, first checking for a known dynamic-library object
/// header (ELF, Mach-O/fat Mach-O, or PE), then scanning for the two required
/// Dylint plugin exports (`dylint_version` and `register_lints`). If both are
/// present the library is considered ABI-compatible; otherwise it is flagged
/// as a mismatch.
///
/// This avoids any external tool dependency (no `nm` required) while rejecting
/// arbitrary text files that happen to contain the marker names.
fn abi_is_compatible(path: &Path) -> bool {
    const MARKERS: [&[u8]; 2] = [b"dylint_version", b"register_lints"];

    std::fs::read(path).is_ok_and(|bytes| {
        looks_like_dynamic_library(&bytes)
            && MARKERS
                .iter()
                .all(|marker| bytes.windows(marker.len()).any(|window| window == *marker))
    })
}

fn looks_like_dynamic_library(bytes: &[u8]) -> bool {
    const MAGIC_PREFIXES: [&[u8]; 9] = [
        b"\x7fELF",
        b"MZ",
        b"\xfe\xed\xfa\xce",
        b"\xfe\xed\xfa\xcf",
        b"\xce\xfa\xed\xfe",
        b"\xcf\xfa\xed\xfe",
        b"\xca\xfe\xba\xbe",
        b"\xca\xfe\xba\xbf",
        b"\xbe\xba\xfe\xca",
    ];

    MAGIC_PREFIXES.iter().any(|magic| bytes.starts_with(magic))
}

/// Produce a complete doctor report for the given scope.
#[must_use]
pub fn report(scope: GateScope) -> DoctorReport {
    let tools =
        tool_configs_for_scope(scope).into_iter().flat_map(probe_config).collect::<Vec<_>>();
    DoctorReport::new(scope, tools)
}

/// Return the doctor report for a scope.
///
/// # Errors
///
/// Returns [`OutputError::ComponentUnavailable`] if the compiled doctor matrix
/// is empty, which would mean the output component is misconfigured.
pub fn doctor_report(scope: GateScope) -> Result<DoctorReport, OutputError> {
    let report = report(scope);
    if report.tools.is_empty() {
        Err(OutputError::component_unavailable(OutputComponent::Doctor))
    } else {
        Ok(report)
    }
}
