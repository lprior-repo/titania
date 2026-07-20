//! v1.5 proof-domain newtypes: [`KaniHarnessId`], [`MutantId`],
//! [`MutantOperator`], and [`ToolKind`].
//!
//! Each constructor returns a `Result<Self, _>` with a typed error variant
//! so the core never surfaces `Result<T, String>`. All identifiers round-trip
//! through serde as their inner canonical form.

use core::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::error::{KaniHarnessIdError, MutantIdError, MutantOperatorError, PathSegmentError};

/// Maximum allowed length of a Kani harness identifier.
///
/// Matches the proof-domain cap chosen by the v1.5 contract so a hostile
/// harness name cannot blow up `PROOF_KANI_<NAME>` rule-id construction.
pub const KANI_HARNESS_ID_MAX_LEN: usize = 96;

/// Maximum allowed length of a [`MutantId`] package segment.
///
/// Mirrors [`KANI_HARNESS_ID_MAX_LEN`] for the same reason: keep every
/// downstream rule-id derivation (`PROOF_MUT_<pkg>::...`) within a fixed
/// bound so a hostile package name cannot blow up filesystem key widths.
pub const MUTANT_PKG_MAX_LEN: usize = 96;

/// Maximum allowed length of a [`MutantId`] relative path segment.
///
/// Larger than the package bound because real Rust workspaces carry deep
/// `src/...` paths; the bound is still small enough that any downstream
/// string concatenation remains statically bounded.
pub const MUTANT_PATH_MAX_LEN: usize = 512;

/// Validated Kani harness identifier.
///
/// Format: `^[a-zA-Z][a-zA-Z0-9_]*$` — an ASCII letter followed by zero or
/// more ASCII letters, digits, or underscores; total length capped at
/// [`KANI_HARNESS_ID_MAX_LEN`]. Mixed-case input is accepted; uppercasing
/// is a shell concern that lives in `titania-lanes/run_lane_kani`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KaniHarnessId(String);

impl Serialize for KaniHarnessId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for KaniHarnessId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::new(&raw).map_err(serde::de::Error::custom)
    }
}

impl KaniHarnessId {
    /// Construct a [`KaniHarnessId`] from any candidate string.
    ///
    /// # Errors
    /// - [`KaniHarnessIdError::Empty`] when `s` is empty.
    /// - [`KaniHarnessIdError::TooLong`] when `s.len() > KANI_HARNESS_ID_MAX_LEN`.
    /// - [`KaniHarnessIdError::LeadingNonLetter`] when `s`'s first byte is
    ///   anything other than an ASCII letter (covers leading underscore,
    ///   leading digit, and leading non-ASCII bytes).
    /// - [`KaniHarnessIdError::NotAscii`] when a byte at offset ≥ 1 falls
    ///   outside `[A-Za-z0-9_]`.
    pub fn new(s: &str) -> Result<Self, KaniHarnessIdError> {
        check_khi(s)?;
        Ok(Self(s.to_owned()))
    }

    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for KaniHarnessId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for KaniHarnessId {
    type Err = KaniHarnessIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

/// # Errors
/// - [`KaniHarnessIdError::Empty`] when `s` is empty.
/// - [`KaniHarnessIdError::TooLong`] when `s` exceeds the max length.
/// - [`KaniHarnessIdError::LeadingNonLetter`] when `s`'s first byte is
///   anything other than an ASCII letter (`[a-zA-Z]`).
/// - [`KaniHarnessIdError::NotAscii`] when a byte at offset ≥ 1 falls
///   outside `[A-Za-z0-9_]`.
fn check_khi(s: &str) -> Result<(), KaniHarnessIdError> {
    if s.is_empty() {
        return Err(KaniHarnessIdError::Empty);
    }
    if s.len() > KANI_HARNESS_ID_MAX_LEN {
        return Err(KaniHarnessIdError::TooLong(s.len()));
    }
    let bytes = s.as_bytes();
    let Some(first) = bytes.first().copied() else {
        return Err(KaniHarnessIdError::Empty);
    };
    if !first.is_ascii_alphabetic() {
        return Err(KaniHarnessIdError::LeadingNonLetter { byte: first });
    }
    bytes.iter().enumerate().skip(1).try_for_each(|(offset, byte)| check_khi_byte(*byte, offset))
}

/// # Errors
/// - [`KaniHarnessIdError::NotAscii`] when `byte` is outside `[A-Za-z0-9_]`.
fn check_khi_byte(byte: u8, offset: usize) -> Result<(), KaniHarnessIdError> {
    is_khi_byte(byte).then_some(()).ok_or(KaniHarnessIdError::NotAscii { byte, offset })
}

const fn is_khi_byte(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte.is_ascii_digit() || byte == b'_'
}

/// Closed-set of cargo-mutants operators recognized by the v1.5 mutants
/// lane.
///
/// New operators require a contract amendment; the [`MutantId::parse`]
/// constructor refuses anything outside this set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutantOperator {
    /// `==` replaced with `!=`.
    EqualReplace,
    /// `!=` replaced with `==`.
    NotInserted,
    /// `&&` replaced with `||`.
    AndOr,
    /// Integer literal `+1`.
    IntegerPlusOne,
    /// Integer literal `-1`.
    IntegerMinusOne,
    /// Arithmetic operator flip (`+ <-> -`, `* <-> /`).
    ArithmeticOpFlip,
    /// Replace a method call with the type's `Default::default()`.
    DefaultReplace,
    /// Remove a boolean negation (`!x` → `x`).
    RemoveNegation,
}

impl MutantOperator {
    /// Canonical lowercase `snake_case` form matching the serde rename.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EqualReplace => "equal_replace",
            Self::NotInserted => "not_inserted",
            Self::AndOr => "and_or",
            Self::IntegerPlusOne => "integer_plus_one",
            Self::IntegerMinusOne => "integer_minus_one",
            Self::ArithmeticOpFlip => "arithmetic_op_flip",
            Self::DefaultReplace => "default_replace",
            Self::RemoveNegation => "remove_negation",
        }
    }

    /// Resolve an operator literal to its closed-set enum value.
    ///
    /// Returns `None` when the literal is not in the recognised set so the
    /// caller can produce a typed [`MutantOperatorError::Unknown`] (or the
    /// outer [`MutantIdError::UnknownOperator`] when called through the
    /// [`MutantId`] parser).
    #[must_use]
    pub fn from_wire(name: &str) -> Option<Self> {
        match name {
            "equal_replace" => Some(Self::EqualReplace),
            "not_inserted" => Some(Self::NotInserted),
            "and_or" => Some(Self::AndOr),
            "integer_plus_one" => Some(Self::IntegerPlusOne),
            "integer_minus_one" => Some(Self::IntegerMinusOne),
            "arithmetic_op_flip" => Some(Self::ArithmeticOpFlip),
            "default_replace" => Some(Self::DefaultReplace),
            "remove_negation" => Some(Self::RemoveNegation),
            _ => None,
        }
    }
}

impl FromStr for MutantOperator {
    type Err = MutantOperatorError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_wire(s).ok_or_else(|| MutantOperatorError::Unknown(s.to_owned()))
    }
}

/// Stable identifier for one cargo-mutants mutation.
///
/// Wire format: `<pkg>::<rel-path>:<line>:<col>:<operator>`. The four `:`s
/// are positional separators, so the `rel-path` segment itself must not
/// contain a `:` — such forms are ambiguous and rejected by
/// [`MutantId::parse`].
///
/// Identifiers round-trip through serde as their inner canonical string.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MutantId(String);

impl Serialize for MutantId {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for MutantId {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        Self::parse(&raw).map_err(serde::de::Error::custom)
    }
}

impl MutantId {
    /// Construct a [`MutantId`] from its constituent parts.
    ///
    /// # Errors
    /// - [`MutantIdError::EmptyPackage`] / [`MutantIdError::EmptyPath`] when
    ///   `package` or `rel_path` is empty.
    /// - [`MutantIdError::PackageTooLong`] / [`MutantIdError::PathTooLong`]
    ///   when the corresponding segment exceeds the bounded character cap.
    /// - [`MutantIdError::PackageInvalid`] / [`MutantIdError::PathInvalid`]
    ///   when the segment violates the bounded character policy (NUL,
    ///   control byte, backslash, embedded `:`, drive prefix, UNC form,
    ///   `..` component, or absolute path).
    /// - [`MutantIdError::PathAbsolute`] when `rel_path` starts with `/`.
    /// - [`MutantIdError::LineNotPositive`] / [`MutantIdError::ColNotPositive`]
    ///   when line/col are zero.
    pub fn new(
        package: &str,
        rel_path: &str,
        line: u32,
        col: u32,
        operator: MutantOperator,
    ) -> Result<Self, MutantIdError> {
        check_mutant_shape(package, rel_path, line, col)?;
        Ok(Self(format!("{package}::{rel_path}:{line}:{col}:{}", operator.as_str())))
    }

    /// Parse the canonical wire form `<pkg>::<rel-path>:<line>:<col>:<operator>`.
    ///
    /// Malformed shapes, unknown operators, ambiguous `:` in the path
    /// segment, and non-numeric line/col return a typed [`MutantIdError`]
    /// instead of panicking.
    ///
    /// # Errors
    /// - [`MutantIdError::MissingSeparator`] when `s` does not contain `::`.
    /// - [`MutantIdError::MissingOperator`] when the `:operator` suffix is
    ///   absent (fewer than four `:` after the `::`).
    /// - [`MutantIdError::UnknownOperator`] when the operator literal is
    ///   outside the closed set.
    /// - [`MutantIdError::PathContainsColon`] when the path segment itself
    ///   contains a `:` (ambiguous under positional parsing).
    /// - [`MutantIdError::LineNotAnInteger`] / [`MutantIdError::ColNotAnInteger`]
    ///   when line/col are missing or non-numeric.
    /// - [`MutantIdError::EmptyPackage`] / [`MutantIdError::EmptyPath`] when
    ///   the parsed `pkg`/`rel_path` components are empty.
    /// - [`MutantIdError::PackageTooLong`] / [`MutantIdError::PathTooLong`]
    ///   when the parsed segment exceeds its bounded character cap.
    /// - [`MutantIdError::PackageInvalid`] / [`MutantIdError::PathInvalid`]
    ///   when a parsed segment violates the bounded character policy.
    /// - [`MutantIdError::PathAbsolute`] when the parsed `rel-path` starts
    ///   with `/`.
    /// - [`MutantIdError::LineNotPositive`] / [`MutantIdError::ColNotPositive`]
    ///   when the parsed value is `0`.
    pub fn parse(s: &str) -> Result<Self, MutantIdError> {
        let (package, rest) =
            s.split_once("::").ok_or_else(|| MutantIdError::MissingSeparator(s.to_owned()))?;

        // The canonical wire form `<pkg>::<path>:<line>:<col>:<operator>`
        // leaves three positional `:` separators inside `rest`. Walk them
        // off from the right with three nested `rsplit_once` calls so each
        // step keeps the str-slice API (no byte-offset arithmetic, no
        // temporary `Vec`). Fewer than three `:`s ⇒ `MissingOperator`.
        let (before_op, operator_name) =
            rest.rsplit_once(':').ok_or_else(|| MutantIdError::MissingOperator(s.to_owned()))?;
        let operator = MutantOperator::from_wire(operator_name)
            .ok_or_else(|| MutantIdError::UnknownOperator(operator_name.to_owned()))?;
        let (before_col, col_str) = before_op
            .rsplit_once(':')
            .ok_or_else(|| MutantIdError::MissingOperator(s.to_owned()))?;
        let (path, line_str) = before_col
            .rsplit_once(':')
            .ok_or_else(|| MutantIdError::MissingOperator(s.to_owned()))?;

        validate_path_segment(path, s)?;
        let line = parse_u32(line_str).ok_or(MutantIdError::LineNotAnInteger)?;
        let col = parse_u32(col_str).ok_or(MutantIdError::ColNotAnInteger)?;
        ensure_positive(line, col)?;
        check_mutant_shape(package, path, line, col)?;
        Ok(Self(format!("{package}::{path}:{line}:{col}:{}", operator.as_str())))
    }

    /// Borrow the inner string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Package prefix (the substring before the first `::`).
    #[must_use]
    pub fn package(&self) -> &str {
        self.0.split_once("::").map_or("", |(pkg, _)| pkg)
    }

    /// Location suffix (`<rel-path>:<line>:<col>:<operator>`).
    #[must_use]
    pub fn location(&self) -> &str {
        self.0.split_once("::").map_or("", |(_, loc)| loc)
    }
}

impl fmt::Display for MutantId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for MutantId {
    type Err = MutantIdError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

/// Reject `:` inside the recovered path segment.
///
/// The canonical wire form fixes the three positional `:` from the right,
/// so an embedded `:` in the path would shift line/col into the path and
/// create an ambiguous partition.
///
/// # Errors
/// - [`MutantIdError::PathContainsColon`] when `path` carries an
///   embedded `:` byte.
fn validate_path_segment(path: &str, original: &str) -> Result<(), MutantIdError> {
    if path.contains(':') {
        Err(MutantIdError::PathContainsColon(original.to_owned()))
    } else {
        Ok(())
    }
}

/// Parse an unsigned-decimal integer literal via a left fold.
///
/// `None` on empty input, an out-of-range digit, or a multiplication /
/// addition overflow; `Some(0)` is preserved so the caller can
/// distinguish "value present and zero" from "missing or non-numeric".
fn parse_u32(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    s.bytes().try_fold(0u32, |acc, byte| {
        let digit = u32::from(byte.checked_sub(b'0')?);
        (digit <= 9).then_some(())?;
        acc.checked_mul(10)?.checked_add(digit)
    })
}

/// # Errors
/// - [`MutantIdError::LineNotPositive`] when `line == 0`.
/// - [`MutantIdError::ColNotPositive`] when `col == 0`.
const fn ensure_positive(line: u32, col: u32) -> Result<(), MutantIdError> {
    if line == 0 {
        return Err(MutantIdError::LineNotPositive);
    }
    if col == 0 {
        return Err(MutantIdError::ColNotPositive);
    }
    Ok(())
}

/// Validate the full [`MutantId`] shape: package / path character policy,
/// bounded length, 1-based line/column.
///
/// Both segments share the same hostile-input rejection rules
/// (see [`PathSegmentError`]); the only segment-specific deviation is
/// [`PathSegmentError::LeadingSlash`], which we surface as the legacy
/// [`MutantIdError::PathAbsolute`] to preserve the public diagnostic for
/// path inputs.
///
/// # Errors
/// - [`MutantIdError::EmptyPackage`] / [`MutantIdError::EmptyPath`] when
///   either segment is empty.
/// - [`MutantIdError::PackageTooLong`] / [`MutantIdError::PathTooLong`]
///   when the segment exceeds its bounded cap.
/// - [`MutantIdError::PackageInvalid`] / [`MutantIdError::PathInvalid`] when
///   the segment carries a forbidden character.
/// - [`MutantIdError::PathAbsolute`] when `rel_path` starts with `/`.
/// - [`MutantIdError::LineNotPositive`] / [`MutantIdError::ColNotPositive`]
///   for zero indices.
fn check_mutant_shape(
    package: &str,
    rel_path: &str,
    line: u32,
    col: u32,
) -> Result<(), MutantIdError> {
    check_pkg(package)?;
    check_path(rel_path)?;
    if line == 0 {
        return Err(MutantIdError::LineNotPositive);
    }
    if col == 0 {
        return Err(MutantIdError::ColNotPositive);
    }
    Ok(())
}

/// # Errors
/// - [`MutantIdError::EmptyPackage`] when `pkg` is empty.
/// - [`MutantIdError::PackageTooLong`] when `pkg.len() > MUTANT_PKG_MAX_LEN`.
/// - [`MutantIdError::PackageInvalid`] when `pkg` violates the bounded
///   character policy.
fn check_pkg(pkg: &str) -> Result<(), MutantIdError> {
    if pkg.is_empty() {
        return Err(MutantIdError::EmptyPackage);
    }
    if pkg.len() > MUTANT_PKG_MAX_LEN {
        return Err(MutantIdError::PackageTooLong { found: pkg.len(), max: MUTANT_PKG_MAX_LEN });
    }
    scan_segment(pkg).map_err(MutantIdError::PackageInvalid)
}

/// # Errors
/// - [`MutantIdError::EmptyPath`] when `path` is empty.
/// - [`MutantIdError::PathTooLong`] when `path.len() > MUTANT_PATH_MAX_LEN`.
/// - [`MutantIdError::PathAbsolute`] when `path` starts with `/` (the
///   segment-specific `LeadingSlash` form).
/// - [`MutantIdError::PathInvalid`] when `path` violates the bounded
///   character policy.
fn check_path(path: &str) -> Result<(), MutantIdError> {
    if path.is_empty() {
        return Err(MutantIdError::EmptyPath);
    }
    if path.len() > MUTANT_PATH_MAX_LEN {
        return Err(MutantIdError::PathTooLong { found: path.len(), max: MUTANT_PATH_MAX_LEN });
    }
    if path.starts_with('/') {
        return Err(MutantIdError::PathAbsolute);
    }
    scan_segment(path).map_err(MutantIdError::PathInvalid)
}

/// Apply the shared hostile-input policy to one segment.
///
/// Both package and path use this same scanner so the rule set stays in
/// lock-step; the caller decides how to wrap the resulting
/// [`PathSegmentError`].
///
/// # Errors
/// - [`PathSegmentError::DriveAbsolute`] when the segment starts with a
///   `<letter>:` prefix.
/// - [`PathSegmentError::UncForm`] when the segment starts with `\\…` or
///   `//…`.
/// - [`PathSegmentError::ContainsBackslash`] when the segment carries a
///   `\` anywhere.
/// - [`PathSegmentError::ContainsColon`] when the segment carries a `:`
///   anywhere.
/// - [`PathSegmentError::ContainsNull`] when the segment carries a NUL
///   byte.
/// - [`PathSegmentError::ControlByte`] when the segment carries any other
///   ASCII control byte (0x01–0x1F, 0x7F).
/// - [`PathSegmentError::ContainsDotDot`] when any `/`-separated component
///   is exactly `..`.
fn scan_segment(s: &str) -> Result<(), PathSegmentError> {
    if let Some(prefix) = drive_prefix(s) {
        return Err(PathSegmentError::DriveAbsolute(prefix));
    }
    if s.starts_with("\\\\") || s.starts_with("//") {
        return Err(PathSegmentError::UncForm);
    }
    let bytes = s.as_bytes();
    if bytes.contains(&b'\\') {
        return Err(PathSegmentError::ContainsBackslash);
    }
    if bytes.contains(&b':') {
        return Err(PathSegmentError::ContainsColon);
    }
    if bytes.contains(&0) {
        return Err(PathSegmentError::ContainsNull);
    }
    if let Some(&bad) = bytes.iter().find(|&&b| b < 0x20 || b == 0x7F) {
        return Err(PathSegmentError::ControlByte(bad));
    }
    if s.split('/').any(|seg| seg == "..") {
        return Err(PathSegmentError::ContainsDotDot);
    }
    Ok(())
}

/// Detect a Windows drive-absolute prefix (`<letter>:`) as the first two
/// bytes of `s` and return its canonical rendering.
///
/// Returns `None` when the segment does not start with a drive prefix.
fn drive_prefix(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let (&first, second) = bytes.first().zip(bytes.get(1))?;
    if !first.is_ascii_alphabetic() || second != &b':' {
        return None;
    }
    // Both bytes are single-byte ASCII, so the 2-char slice is valid UTF-8
    // whenever the bytes say so.
    Some(s.get(..2).map_or_else(|| format!("{first}:"), ToString::to_string))
}

/// Tool identifier for [`crate::outcome::SkipReason::ToolUnavailable`].
///
/// Closed set: only the tools the v1.5 contract recognises. New tools require
/// a contract amendment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolKind {
    /// `cargo-kani` model-checker.
    CargoKani,
    /// `cargo-mutants` mutation tester.
    CargoMutants,
}

impl ToolKind {
    /// Canonical kebab-case form matching the serde rename.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CargoKani => "cargo-kani",
            Self::CargoMutants => "cargo-mutants",
        }
    }
}
