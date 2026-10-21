use std::{
    process::{Command, Stdio},
    sync::OnceLock,
};

use super::constants::MUTANTS_VERSION_FLOOR_MAJOR;

#[must_use]
pub(super) fn detect_cargo_mutants_version() -> Option<String> {
    let output = Command::new("cargo")
        .arg("mutants")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .output();
    let Ok(output) = output else {
        return None;
    };
    if !output.status.success() {
        return None;
    }
    let Ok(stdout) = std::str::from_utf8(&output.stdout) else {
        return None;
    };
    Some(stdout.to_owned())
}

#[must_use]
pub(super) fn parse_cargo_mutants_major(stdout: &str) -> Option<u32> {
    let trimmed = stdout.trim();
    let after_prefix = trimmed.strip_prefix("cargo-mutants ")?;
    let major_str = after_prefix.split('.').next()?;
    let Ok(major) = major_str.parse::<u32>() else {
        return None;
    };
    Some(major)
}

#[must_use]
pub(super) const fn major_meets_floor(major: u32) -> bool {
    major >= MUTANTS_VERSION_FLOOR_MAJOR
}

#[must_use]
pub(super) fn probe_systemd_run() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    AVAILABLE.get_or_init(detect_systemd_run).to_owned()
}

fn detect_systemd_run() -> bool {
    Command::new("systemd-run")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[cfg(test)]
mod tests {
    use super::{major_meets_floor, parse_cargo_mutants_major};

    #[test]
    fn parse_major_extracts_27_from_cargo_mutants_27_line() {
        let major = parse_cargo_mutants_major("cargo-mutants 27.0.0\n")
            .unwrap_or_else(|| panic!("cargo-mutants 27 prefix must parse to a major version"));
        assert_eq!(major, 27);
    }

    #[test]
    fn parse_major_extracts_27_from_no_trailing_newline() {
        let major = parse_cargo_mutants_major("cargo-mutants 27.0.0")
            .unwrap_or_else(|| panic!("cargo-mutants 27 prefix must parse to a major version"));
        assert_eq!(major, 27);
    }

    #[test]
    fn parse_major_rejects_non_cargo_mutants_prefix() {
        assert_eq!(parse_cargo_mutants_major(""), None);
        assert_eq!(parse_cargo_mutants_major("no such subcommand: 'mutants'"), None);
        assert_eq!(parse_cargo_mutants_major("cargo-mutants"), None);
        assert_eq!(parse_cargo_mutants_major("cargo-mutants xyz"), None);
    }

    #[test]
    fn parse_major_rejects_negative_or_non_decimal_input() {
        assert_eq!(parse_cargo_mutants_major("cargo-mutants -1.0.0"), None);
        assert_eq!(parse_cargo_mutants_major("cargo-mutants v1.0.0"), None);
    }

    #[test]
    fn major_meets_floor_accepts_known_floor_and_above() {
        assert!(major_meets_floor(25));
        assert!(major_meets_floor(27));
        assert!(major_meets_floor(99));
    }

    #[test]
    fn major_meets_floor_rejects_below_floor() {
        assert!(!major_meets_floor(24));
        assert!(!major_meets_floor(0));
    }
}
