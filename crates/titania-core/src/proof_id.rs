use core::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};

use crate::error::{KaniHarnessIdError, MutantIdError, MutantOperatorError, PathSegmentError};

mod kernel;

pub const KANI_HARNESS_ID_MAX_LEN: usize = 96;

pub const MUTANT_PKG_MAX_LEN: usize = 96;

pub const MUTANT_PATH_MAX_LEN: usize = 512;

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
    pub fn new(s: &str) -> Result<Self, KaniHarnessIdError> {
        check_khi(s)?;
        Ok(Self(s.to_owned()))
    }

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

fn check_khi_byte(byte: u8, offset: usize) -> Result<(), KaniHarnessIdError> {
    kernel::is_khi_byte(byte).then_some(()).ok_or(KaniHarnessIdError::NotAscii { byte, offset })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MutantOperator {
    EqualReplace,
    NotInserted,
    AndOr,
    IntegerPlusOne,
    IntegerMinusOne,
    ArithmeticOpFlip,
    DefaultReplace,
    RemoveNegation,
}

impl MutantOperator {
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

    pub fn parse(s: &str) -> Result<Self, MutantIdError> {
        let (package, rest) =
            s.split_once("::").ok_or_else(|| MutantIdError::MissingSeparator(s.to_owned()))?;

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

    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }

    #[must_use]
    pub fn package(&self) -> &str {
        self.0.split_once("::").map_or("", |(pkg, _)| pkg)
    }

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

fn validate_path_segment(path: &str, original: &str) -> Result<(), MutantIdError> {
    if path.contains(':') {
        Err(MutantIdError::PathContainsColon(original.to_owned()))
    } else {
        Ok(())
    }
}

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

const fn ensure_positive(line: u32, col: u32) -> Result<(), MutantIdError> {
    if line == 0 {
        return Err(MutantIdError::LineNotPositive);
    }
    if col == 0 {
        return Err(MutantIdError::ColNotPositive);
    }
    Ok(())
}

fn check_mutant_shape(
    package: &str,
    rel_path: &str,
    line: u32,
    col: u32,
) -> Result<(), MutantIdError> {
    check_pkg(package)?;
    check_path(rel_path)?;
    ensure_positive(line, col)
}

fn check_pkg(pkg: &str) -> Result<(), MutantIdError> {
    if pkg.is_empty() {
        return Err(MutantIdError::EmptyPackage);
    }
    if pkg.len() > MUTANT_PKG_MAX_LEN {
        return Err(MutantIdError::PackageTooLong { found: pkg.len(), max: MUTANT_PKG_MAX_LEN });
    }
    scan_segment(pkg).map_err(MutantIdError::PackageInvalid)
}

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

fn drive_prefix(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let (&first, second) = bytes.first().zip(bytes.get(1))?;
    if !first.is_ascii_alphabetic() || second != &b':' {
        return None;
    }
    Some(s.get(..2).map_or_else(|| format!("{first}:"), ToString::to_string))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolKind {
    CargoKani,
    CargoMutants,
}

impl ToolKind {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CargoKani => "cargo-kani",
            Self::CargoMutants => "cargo-mutants",
        }
    }
}
