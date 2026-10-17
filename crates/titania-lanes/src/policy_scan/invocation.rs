//! Effective-invocation lint-ceiling guard.
//!
//! `rustc --cap-lints <level>` ceilings every lint level, so a single
//! ceiling token anywhere in the effective compiler invocation neuters the
//! lane's `-D` denies while the tool still exits 0. Environment, encoded,
//! and config channels are already rejected by sibling scanners and dropped
//! by the spawn scrub; this module owns the remaining channel —
//! caller-supplied extra arguments appended to the protected cargo argv —
//! as a pure predicate the runner enforces before spawning.

use thiserror::Error;

/// Bare lint-ceiling flag (`--cap-lints <level>`).
const CAP_LINTS_FLAG: &str = "--cap-lints";
/// Joined lint-ceiling assignment (`--cap-lints=<level>`).
const CAP_LINTS_ASSIGN: &str = "--cap-lints=";

/// Rejection for a ceiling-carrying invocation fragment.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum InvocationError {
    /// An extra argument would cap mandatory lint severity.
    #[error("extra argument {arg:?} would cap mandatory lint severity below the lane denies")]
    CapLints {
        /// The offending token, echoed so callers can correct the argv.
        arg: String,
    },
}

/// `true` when `arg` is a rustc lint-ceiling token.
///
/// Matches the bare flag (whose level arrives as the next token) and the
/// joined assignment form. Matching is exact and case-sensitive because
/// rustc flags are; lookalikes such as `--cap-lints-allow` do not match.
#[must_use]
pub fn is_cap_lints_flag(arg: &str) -> bool {
    arg == CAP_LINTS_FLAG || arg.starts_with(CAP_LINTS_ASSIGN)
}

/// Reject ceiling-carrying extra arguments before they reach a spawn.
///
/// Scans every element, including tokens after a `--` separator, because
/// cargo forwards those to the compiler driver unchanged.
///
/// # Errors
/// Returns [`InvocationError::CapLints`] naming the first ceiling token.
pub fn validate_extra_args(extra_args: &[String]) -> Result<(), InvocationError> {
    let ceiling = extra_args.iter().find(|arg| is_cap_lints_flag(arg));
    ceiling.map_or(Ok(()), |arg| Err(InvocationError::CapLints { arg: arg.clone() }))
}
