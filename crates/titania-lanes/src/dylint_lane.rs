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
use toml_edit::Item;

use crate::{LaneReport, RuleId};

const LIB_TITANIA_DYLINT: &str = "libtitania_dylint";
const RULE_DYLINT_INFRA: &str = "DYLINT_INFRA_FAILURE";
const TITANIA_DYLINT_LIB_ENV: &str = "TITANIA_DYLINT_LIB";
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
    /// Both tools are available; the lane can proceed.
    Ready,
    /// Infrastructure probe failed with typed details.
    Infra(LaneFailure, LaneReport),
}

impl DylintProbe {
    /// Whether the probe indicates the lane can proceed.
    #[must_use]
    pub const fn is_ready(&self) -> bool {
        matches!(self, Self::Ready)
    }

    /// Borrow failure details when the probe did not succeed.
    #[must_use]
    pub const fn failure(&self) -> Option<&LaneFailure> {
        match self {
            Self::Ready => None,
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
        LibraryProbe::Compatible => DylintProbe::Ready,
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
    Compatible,
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
    if let Some(path) = env_library_path() {
        return probe_library_path(path);
    }
    if workspace_metadata_dylint_configured(target.as_std_path()) {
        return LibraryProbe::Compatible;
    }
    sibling_library_path().map_or(LibraryProbe::Absent, probe_library_path)
}

fn env_library_path() -> Option<PathBuf> {
    std::env::var_os(TITANIA_DYLINT_LIB_ENV).map(PathBuf::from)
}

fn probe_library_path(path: PathBuf) -> LibraryProbe {
    if !path.is_file() {
        return LibraryProbe::Incompatible {
            path,
            reason: String::from("configured library path is not a file"),
        };
    }
    abi_check(&path)
        .map_or(LibraryProbe::Compatible, |reason| LibraryProbe::Incompatible { path, reason })
}

fn workspace_metadata_dylint_configured(root: &Path) -> bool {
    std::fs::read_to_string(root.join("Cargo.toml"))
        .ok()
        .and_then(|content| content.parse::<toml_edit::DocumentMut>().ok())
        .is_some_and(|document| metadata_libraries(&document))
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
        .is_some()
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
    // ELF: 0x7F 'E' 'L' 'F'
    if head[0] == 0x7f && head[1] == b'E' && head[2] == b'L' && head[3] == b'F' {
        return true;
    }
    // Mach-O variants
    if head[0] == 0xfe && head[1] == 0xed && head[2] == 0xfa && (head[3] == 0xce || head[3] == 0xcf)
    {
        return true;
    }
    if head[0] == 0xce && head[1] == 0xfa && head[2] == 0xed && head[3] == 0xfe {
        return true;
    }
    if head[0] == 0xcf && head[1] == 0xfa && head[2] == 0xed && head[3] == 0xfe {
        return true;
    }
    // PE / DOS MZ ('M' 'Z')
    if head[0] == b'M' && head[1] == b'Z' {
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
