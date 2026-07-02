//! Checks `.beads/metadata.json` mode without assuming every target project
//! must use the same Beads backend topology.
//!
//! The original CI lane required server-mode Dolt everywhere.
//! Titania itself currently uses embedded Dolt, so this lane now parses the
//! metadata into typed policy values and rejects malformed/contradictory
//! states while treating embedded mode as an explicit supported outcome.
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![forbid(unsafe_code)]

use std::{fs, io, io::Write};

use serde_json::Value;
use titania_lanes::{Finding, LaneExit, LaneReport, RuleId, RuleIdError, exit};

const METADATA_PATH: &str = ".beads/metadata.json";
const EMBEDDED_MARKER: &str = ".beads/embeddeddolt";

const RULE_BACKEND: &str = "BEADS_BACKEND_001";
const RULE_DOLT_MODE: &str = "BEADS_MODE_001";
const RULE_DOLT_PORT: &str = "BEADS_DOLT_PORT_001";
const RULE_EMBEDDED_MARKER: &str = "BEADS_EMBEDDED_MARKER_001";
const RULE_METADATA_MISSING: &str = "BEADS_METADATA_MISSING_001";
const RULE_METADATA_PARSE: &str = "BEADS_METADATA_PARSE_001";

struct BeadsRules {
    backend: RuleId,
    dolt_mode: RuleId,
    dolt_port: RuleId,
    embedded_marker: RuleId,
    metadata_missing: RuleId,
    metadata_parse: RuleId,
}

impl BeadsRules {
    /// Construct rule identifiers for Beads metadata validation.
    ///
    /// # Errors
    ///
    /// Returns [`RuleIdError`] when a configured rule identifier is invalid.
    fn new() -> Result<Self, RuleIdError> {
        Ok(Self {
            backend: RuleId::new(RULE_BACKEND)?,
            dolt_mode: RuleId::new(RULE_DOLT_MODE)?,
            dolt_port: RuleId::new(RULE_DOLT_PORT)?,
            embedded_marker: RuleId::new(RULE_EMBEDDED_MARKER)?,
            metadata_missing: RuleId::new(RULE_METADATA_MISSING)?,
            metadata_parse: RuleId::new(RULE_METADATA_PARSE)?,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Backend {
    Dolt,
    Other,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DoltMode {
    Server,
    Embedded,
    Other,
    Missing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct BeadsMetadata {
    backend: Backend,
    mode: DoltMode,
    pins_server_port: bool,
}

impl BeadsMetadata {
    /// Parse Beads metadata JSON into typed policy fields.
    ///
    /// # Errors
    ///
    /// Returns the serde JSON parse error when the metadata is not valid JSON.
    fn parse(text: &str) -> Result<Self, serde_json::Error> {
        let value = serde_json::from_str::<Value>(text)?;
        Ok(Self {
            backend: backend_from(value_text(&value, "backend")),
            mode: mode_from(value_text(&value, "dolt_mode")),
            pins_server_port: value.get("dolt_server_port").is_some(),
        })
    }

    fn check(self, rules: &BeadsRules, report: &mut LaneReport) {
        check_backend(self.backend, rules, report);
        check_mode(self.mode, rules, report);
        check_port_pin(self.pins_server_port, rules, report);
    }
}

fn value_text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn backend_from(value: Option<&str>) -> Backend {
    match value {
        Some("dolt") => Backend::Dolt,
        Some(_) => Backend::Other,
        None => Backend::Missing,
    }
}

fn mode_from(value: Option<&str>) -> DoltMode {
    match value {
        Some("server") => DoltMode::Server,
        Some("embedded") => DoltMode::Embedded,
        Some(_) => DoltMode::Other,
        None => DoltMode::Missing,
    }
}

fn check_backend(backend: Backend, rules: &BeadsRules, report: &mut LaneReport) {
    if backend != Backend::Dolt {
        report.push(Finding::new(
            rules.backend.clone(),
            METADATA_PATH,
            0,
            ".beads/metadata.json must declare backend \"dolt\"",
        ));
    }
}

fn check_mode(mode: DoltMode, rules: &BeadsRules, report: &mut LaneReport) {
    match mode {
        DoltMode::Server | DoltMode::Embedded => {}
        DoltMode::Missing => report.push(Finding::new(
            rules.dolt_mode.clone(),
            METADATA_PATH,
            0,
            ".beads/metadata.json must declare dolt_mode",
        )),
        DoltMode::Other => report.push(Finding::new(
            rules.dolt_mode.clone(),
            METADATA_PATH,
            0,
            ".beads/metadata.json contains unsupported dolt_mode",
        )),
    }
}

fn check_port_pin(pins_server_port: bool, rules: &BeadsRules, report: &mut LaneReport) {
    if pins_server_port {
        report.push(Finding::new(
            rules.dolt_port.clone(),
            METADATA_PATH,
            0,
            "do not pin dolt_server_port in metadata; bd owns runtime routing",
        ));
    }
}

fn check_embedded_marker(mode: DoltMode, rules: &BeadsRules, report: &mut LaneReport) {
    if mode == DoltMode::Server && fs::metadata(EMBEDDED_MARKER).is_ok() {
        report.push(Finding::new(
            rules.embedded_marker.clone(),
            EMBEDDED_MARKER,
            0,
            ".beads/embeddeddolt conflicts with server-mode metadata",
        ));
    }
}

fn check_metadata(
    text: &str,
    rules: &BeadsRules,
    report: &mut LaneReport,
) -> Option<BeadsMetadata> {
    report.record_scan();
    match BeadsMetadata::parse(text) {
        Ok(metadata) => {
            metadata.check(rules, report);
            Some(metadata)
        }
        Err(error) => {
            report.push(Finding::new(
                rules.metadata_parse.clone(),
                METADATA_PATH,
                0,
                format!(".beads/metadata.json is not valid JSON: {error}"),
            ));
            None
        }
    }
}

fn main() -> std::process::ExitCode {
    let rules = match BeadsRules::new() {
        Ok(rules) => rules,
        Err(error) => {
            return exit_after_stderr_line(
                &format!("[check-beads-server-mode] rule id configuration error: {error}"),
                LaneExit::Failure,
            );
        }
    };
    let mut report = LaneReport::new();

    let metadata = match fs::read_to_string(METADATA_PATH) {
        Ok(text) => text,
        Err(error) => return metadata_missing_exit(&rules, &mut report, &error),
    };

    if let Some(parsed) = check_metadata(&metadata, &rules, &mut report) {
        check_embedded_marker(parsed.mode, &rules, &mut report);
    }

    if write_stderr(&report.render()).is_err() {
        return exit(LaneExit::Failure);
    }
    if report.is_clean() {
        exit_after_stderr_line("beads metadata mode check passed", LaneExit::Clean)
    } else {
        exit(LaneExit::Violations)
    }
}

fn metadata_missing_exit(
    rules: &BeadsRules,
    report: &mut LaneReport,
    error: &io::Error,
) -> std::process::ExitCode {
    report.push(Finding::new(
        rules.metadata_missing.clone(),
        METADATA_PATH,
        0,
        format!(".beads/metadata.json is missing: {error}"),
    ));
    match write_stderr(&report.render()) {
        Ok(()) => exit(LaneExit::Violations),
        Err(_) => exit(LaneExit::Failure),
    }
}

/// Write raw text to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())
}

/// Write one line to stderr.
///
/// # Errors
///
/// Returns the underlying stderr write error.
fn write_stderr_line(text: &str) -> io::Result<()> {
    let mut stderr = io::stderr().lock();
    stderr.write_all(text.as_bytes())?;
    stderr.write_all(b"\n")
}

fn exit_after_stderr_line(text: &str, code: LaneExit) -> std::process::ExitCode {
    match write_stderr_line(text) {
        Ok(()) => exit(code),
        Err(_) => exit(LaneExit::Failure),
    }
}
