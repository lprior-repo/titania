//! Dylint lane loader — checks tool availability and reports typed infra failures.
//!
//! This module owns the Dylint lane's pre-flight checks:
//! 1. Is `cargo-dylint` available (via `cargo dylint --help`)? If not,
//!    return [`LaneFailure::Infra`] with `tool = "cargo-dylint"`.
//! 2. Is the `libtitania_dylint` cdylib available and ABI-compatible? If not,
//!    return [`LaneFailure::Infra`] with `tool = "libtitania_dylint"`.
//!
//! No lint logic lives here — only the load / wiring contract.

use titania_core::{LaneFailure, TargetProject};

use crate::{LaneReport, RuleId};

const LIB_TITANIA_DYLINT: &str = "libtitania_dylint";
const RULE_DYLINT_INFRA: &str = "DYLINT_INFRA_FAILURE";
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
    match probe_libtitania_dylint(LIB_TITANIA_DYLINT) {
        LibraryProbe::Compatible => DylintProbe::Ready,
        LibraryProbe::Absent => unavailable_probe(
            LIB_TITANIA_DYLINT,
            format!("{LIB_TITANIA_DYLINT} unavailable: no plugin library found in target dir"),
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
fn probe_libtitania_dylint(lib_name: &str) -> LibraryProbe {
    let target_dir = cargo_target_dir();
    let candidates = [
        format!("debug/{lib_name}.so"),
        format!("debug/{lib_name}.dylib"),
        format!("debug/{lib_name}.dll"),
        format!("release/{lib_name}.so"),
        format!("release/{lib_name}.dylib"),
        format!("release/{lib_name}.dll"),
    ];

    let Some(path) = candidates
        .iter()
        .map(|p| target_dir.join(p))
        .find(|p| p.is_file())
    else {
        return LibraryProbe::Absent;
    };

    match abi_check(&path) {
        Ok(()) => LibraryProbe::Compatible,
        Err(reason) => LibraryProbe::Incompatible { path, reason },
    }
}

/// Minimal ABI sanity check on the resolved library path.
///
/// Today we verify the file is a loadable dynamic library by checking the
/// platform magic (ELF on Linux, Mach-O on macOS, PE on Windows). When the
/// dylint loader evolves to require an exported `dylint_version` symbol, this
/// is the place to extend it. Returns `Ok(())` on success or `Err` carrying
/// a short human-readable reason suitable for [`LaneFailure::Infra.reason`].
///
/// # Errors
/// Returns `Err` when the file cannot be read, when the file is too short to
/// hold a recognised dynamic library magic, or when the leading bytes do not
/// match ELF / Mach-O / PE magic.
fn abi_check(path: &std::path::Path) -> Result<(), String> {
    let bytes = std::fs::read(path)
        .map_err(|e| format!("cannot read library at {}: {e}", path.display()))?;
    let head = bytes.first_chunk::<4>().copied().ok_or_else(|| {
        format!(
            "library at {} is too short ({} bytes) to be a dynamic library",
            path.display(),
            bytes.len()
        )
    })?;
    if !is_dynamic_library_magic(head) {
        return Err(format!(
            "library at {} is not a dynamic library (missing ELF/Mach-O/PE magic; got bytes {:02x?})",
            path.display(),
            head
        ));
    }
    Ok(())
}

/// Return `true` when the 4-byte head of a candidate library file matches the
/// platform magic for an ELF, Mach-O, or PE (DOS MZ) dynamic library.
const fn is_dynamic_library_magic(head: [u8; 4]) -> bool {
    // ELF: 0x7F 'E' 'L' 'F'
    if head[0] == 0x7f && head[1] == b'E' && head[2] == b'L' && head[3] == b'F' {
        return true;
    }
    // Mach-O variants
    if head[0] == 0xfe && head[1] == 0xed && head[2] == 0xfa && (head[3] == 0xce || head[3] == 0xcf) {
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

fn cargo_target_dir() -> std::path::PathBuf {
    std::env::var_os("CARGO_TARGET_DIR").map_or_else(
        || std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target"),
        std::path::PathBuf::from,
    )
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
