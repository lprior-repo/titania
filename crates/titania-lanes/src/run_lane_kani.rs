use std::{
    collections::BTreeMap,
    io::Read,
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::OnceLock,
    time::Duration,
};

use serde::Deserialize;
use thiserror::Error;
use titania_core::{
    CommandEvidence, Digest, Finding, FindingEffect, KaniHarnessId, Lane, LaneEvidence,
    LaneOutcome, Location, OutcomeError, ProcessTermination, RepairHint, RuleId, RuleIdError,
    SkipReason, TargetProject, ToolKind,
};
use wait_timeout::ChildExt;

pub(super) const PER_PACKAGE_TIMEOUT_SECS: u64 = 600;

const PIPE_BYTE_CAP: usize = 8 * 1024 * 1024;

const KANI_TOOL: &str = "cargo-kani";

const MIN_KANI_VERSION: (u64, u64, u64) = (0, 50, 0);

const CGROUP_MEMORY_MAX: &str = "24G";

const FALLBACK_RULE_PASS: &str = "PROOF_KANI_PASS";
const FALLBACK_RULE_FAIL: &str = "PROOF_KANI_FAIL";
const FALLBACK_RULE_BLOCKED: &str = "PROOF_KANI_BLOCKED";
const FALLBACK_RULE_NO_VERDICT: &str = "PROOF_KANI_NO_VERDICT";
const FALLBACK_RULE_UNSUPPORTED: &str = "PROOF_KANI_UNSUPPORTED";
const FALLBACK_RULE_INFRA: &str = "PROOF_KANI_INFRA";
const FALLBACK_RULE_NO_HARNESSES: &str = "PROOF_KANI_NO_HARNESSES";

#[derive(Debug, Error)]
pub(super) enum KaniLaneError {
    #[error("Kani lane requires a Cargo workspace root at {0}")]
    NotACargoWorkspace(String),
    #[error("Kani lane could not construct a rule id: {0}")]
    RuleId(#[source] RuleIdError),
    #[error("cargo-kani inventory failed: {0}")]
    Inventory(#[from] KaniInventoryError),
    #[error("cargo-kani execution failed: {0}")]
    Package(#[from] KaniRunError),
    #[error("cargo kani version probe failed: {0}")]
    VersionProbe(#[from] KaniVersionError),
    #[error("clean-outcome evidence build failed: {0}")]
    Outcome(#[from] OutcomeError),
}

#[derive(Debug, Error)]
pub(super) enum KaniInventoryError {
    #[error("workspace has no crates directory at {0}")]
    NoCratesDir(String),
    #[error("cannot read crates directory at {path}: {source}")]
    CratesDirRead {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("inventory command for crate `{crate_name}` failed: {source}")]
    ListCommand {
        crate_name: String,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "inventory command for crate `{crate_name}` exited with code {code:?}; stderr: {stderr}"
    )]
    ListCommandExit { crate_name: String, code: Option<i32>, stderr: Box<str> },
    #[error("cannot read inventory artifact at {path}: {source}")]
    ArtifactRead {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("cannot parse inventory artifact at {path}: {source}")]
    ArtifactParse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error("cannot remove stale inventory artifact at {path}: {source}")]
    CleanupFailed {
        path: String,
        #[source]
        source: std::io::Error,
    },
}

#[derive(Debug, Error)]
pub(super) enum KaniRunError {
    #[error("spawn failed for {program}: {source}")]
    Spawn {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("wait_timeout failed for {program}: {source}")]
    WaitTimeout {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("kill failed for timed-out {program}: {source}")]
    Kill {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("wait failed for killed {program}: {source}")]
    WaitReap {
        program: String,
        #[source]
        source: std::io::Error,
    },
    #[error("captured {program} {stream} pipe exceeded byte cap of {limit}")]
    PipeCapExceeded { program: String, stream: &'static str, limit: usize },
    #[error("failed to read {program} {stream} pipe: {source}")]
    PipeRead {
        program: String,
        stream: &'static str,
        #[source]
        source: std::io::Error,
    },
    #[error("captured {program} {stream} pipe was not valid UTF-8")]
    PipeNonUtf8 { program: String, stream: &'static str },
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub(super) enum KaniVersionError {
    #[error("cargo kani --version emitted non-UTF-8 stdout")]
    NonUtf8Output,
    #[error("cargo kani --version output was not parseable: {output}")]
    UnparseableOutput { output: Box<str> },
}

#[derive(Debug, Clone)]
struct DetectedCargoKani {
    version: String,
    major: u64,
    minor: u64,
    patch: u64,
}

impl DetectedCargoKani {
    fn meets_floor(&self) -> bool {
        (self.major, self.minor, self.patch) >= MIN_KANI_VERSION
    }
}

#[derive(Debug, Clone)]
struct KaniHarness {
    package: String,
    full_name: String,
    canonical_id: Option<KaniHarnessId>,
}

#[derive(Debug, Default)]
struct LaneRunState {
    worst_exit_code: i32,
    tool_version: String,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum HarnessVerdict {
    Successful,
    Failed,
    Unsupported,
    Unknown,
}

#[derive(Debug, Default)]
struct HarnessVerdictMap {
    verdicts: BTreeMap<String, HarnessVerdict>,
}

impl HarnessVerdictMap {
    fn from_stdout(stdout: &str) -> Self {
        build_verdict_map(stdout)
    }

    fn verdict_for(&self, full_name: &str) -> HarnessVerdict {
        self.verdicts.get(full_name).copied().map_or(HarnessVerdict::Unknown, |v| v)
    }
}

fn build_verdict_map(stdout: &str) -> HarnessVerdictMap {
    let (map, _) = stdout.lines().fold(
        (HarnessVerdictMap::default(), None::<String>),
        |(mut map, mut current), line| {
            absorb_verdict_line(&mut map, &mut current, line);
            (map, current)
        },
    );
    map
}

fn absorb_verdict_line(map: &mut HarnessVerdictMap, current: &mut Option<String>, line: &str) {
    let _ = checking_harness_name(line).map(|name| *current = Some(name.to_owned()));
    let _ = verdict_from_line(line)
        .and_then(|verdict| current.take().map(|name| map.verdicts.insert(name, verdict)));
}

fn checking_harness_name(line: &str) -> Option<&str> {
    let rest = line.trim_start().strip_prefix("Checking harness ")?;
    let name = rest.trim().strip_suffix("...")?.trim();
    if name.is_empty() { None } else { Some(name) }
}

fn verdict_from_line(line: &str) -> Option<HarnessVerdict> {
    let trimmed = line.trim_start();
    let verification = trimmed.strip_prefix("VERIFICATION:")?;
    let verdict = verification.trim().trim_start_matches('-').trim();
    Some(match verdict {
        "SUCCESSFUL" => HarnessVerdict::Successful,
        "FAILED" => HarnessVerdict::Failed,
        other if other.contains("UNSUPPORTED") => HarnessVerdict::Unsupported,
        _ => HarnessVerdict::Unknown,
    })
}

#[derive(Debug, Clone)]
struct PackageCmd {
    executable: String,
    argv: Box<[String]>,
}

#[derive(Debug, Clone)]
struct PackageRun {
    exit_code: Option<i32>,
    timed_out: bool,
    stdout: String,
    cmd: PackageCmd,
    teardown_errors: Vec<String>,
}

impl PackageRun {
    const fn is_successful(&self) -> bool {
        matches!(self.exit_code, Some(code) if code == 0)
    }
}

fn drain_pipe<R: Read>(
    program: &str,
    stream: &'static str,
    pipe: Option<R>,
) -> Result<String, KaniRunError> {
    let Some(mut pipe) = pipe else {
        return Ok(String::new());
    };
    let limit_u64 = u64::try_from(PIPE_BYTE_CAP.saturating_add(1)).map_or(u64::MAX, |n| n);
    let mut limited = pipe.by_ref().take(limit_u64);
    let mut buf: Vec<u8> = Vec::new();
    let _ = limited.read_to_end(&mut buf).map_err(|source| KaniRunError::PipeRead {
        program: program.to_owned(),
        stream,
        source,
    })?;
    if buf.len() > PIPE_BYTE_CAP {
        return Err(KaniRunError::PipeCapExceeded {
            program: program.to_owned(),
            stream,
            limit: PIPE_BYTE_CAP,
        });
    }
    String::from_utf8(buf)
        .map_err(|_source| KaniRunError::PipeNonUtf8 { program: program.to_owned(), stream })
}

pub(super) fn outcome(target: &TargetProject) -> Result<LaneOutcome, KaniLaneError> {
    let workspace_root = target.as_std_path();
    let version_probe = cargo_kani_supported()?;
    if version_probe.is_none() {
        return Ok(LaneOutcome::Skipped {
            reason: SkipReason::ToolUnavailable(ToolKind::CargoKani),
        });
    }
    let tool_version = current_kani_version();
    if !has_workspace_root(target.manifest_path().as_std_path()) {
        return Err(KaniLaneError::NotACargoWorkspace(target.as_std_path().display().to_string()));
    }
    let mut state = LaneRunState { worst_exit_code: 0, tool_version };
    let inventory = match list_kani_harnesses(workspace_root, &mut state) {
        Ok(inv) => inv,
        Err(error) => return inventory_error_outcome(&error),
    };
    if inventory.is_empty() {
        return build_no_harness_outcome(&state).map_err(KaniLaneError::Outcome);
    }
    let groups = group_by_package(&inventory);
    let cgroup_available = probe_systemd_run();
    let mut findings: Vec<Finding> = Vec::new();
    let mut worst_run: Option<PackageRun> = None;
    groups.iter().try_for_each(|(package, harnesses)| {
        let run = run_package(workspace_root, package, cgroup_available)
            .map_err(KaniLaneError::Package)?;
        update_worst_exit(&mut state, &run);
        replace_if_worst(&mut worst_run, &run);
        let verdicts = HarnessVerdictMap::from_stdout(&run.stdout);
        let package_findings = findings_for_package(harnesses, &verdicts, &run)?;
        findings.extend(package_findings);
        Ok::<(), KaniLaneError>(())
    })?;
    if !findings.is_empty() {
        return Ok(LaneOutcome::Findings { findings: findings.into_boxed_slice() });
    }
    let Some(worst) = worst_run else {
        return Err(KaniLaneError::Outcome(OutcomeError::NonZeroExit));
    };
    build_clean_outcome(&state, &worst).map_err(KaniLaneError::Outcome)
}

fn update_worst_exit(state: &mut LaneRunState, run: &PackageRun) {
    if let Some(code) = run.exit_code {
        state.worst_exit_code = state.worst_exit_code.max(code);
    }
}

const fn exit_code_is_greater(prev: &PackageRun, candidate: i32) -> bool {
    match prev.exit_code {
        Some(prev_code) => candidate > prev_code,
        None => true,
    }
}

fn replace_if_worst(slot: &mut Option<PackageRun>, candidate: &PackageRun) {
    let replace = match (slot.as_ref(), candidate.exit_code) {
        (None, Some(_)) => true,
        (Some(prev), Some(code)) => exit_code_is_greater(prev, code),
        _ => false,
    };
    if replace {
        *slot = Some(candidate.clone());
    }
}

fn inventory_error_outcome(error: &KaniInventoryError) -> Result<LaneOutcome, KaniLaneError> {
    if is_subcommand_missing(error) {
        return Ok(LaneOutcome::Skipped {
            reason: SkipReason::ToolUnavailable(ToolKind::CargoKani),
        });
    }
    let finding = infra_finding(&error.to_string())?;
    Ok(LaneOutcome::Findings { findings: vec![finding].into_boxed_slice() })
}

fn build_no_harness_outcome(state: &LaneRunState) -> Result<LaneOutcome, OutcomeError> {
    let rule_id =
        RuleId::new(FALLBACK_RULE_NO_HARNESSES).map_err(|_rule_err| OutcomeError::EmptyArgv)?;
    Ok(LaneOutcome::Findings {
        findings: vec![Finding::informational(
            Lane::Kani,
            rule_id,
            Location::tool(KANI_TOOL.to_owned(), state.tool_version.clone()),
            format!(
                "cargo kani inventory returned no harnesses across the workspace (cargo-kani {})",
                state.tool_version
            ),
            RepairHint::requires_human_review(
                "Workspace contains no `#[kani::proof]` harnesses; add at least one harness \
                 before enabling the Kani lane."
                    .to_owned(),
            ),
        )]
        .into_boxed_slice(),
    })
}

fn is_subcommand_missing(error: &KaniInventoryError) -> bool {
    let text = error.to_string();
    text.contains("no such subcommand") || text.contains("not found")
}

fn findings_for_package(
    harnesses: &[KaniHarness],
    verdicts: &HarnessVerdictMap,
    run: &PackageRun,
) -> Result<Vec<Finding>, KaniLaneError> {
    if run.timed_out || run.exit_code.is_none() {
        return blocked_findings_for_package(harnesses, run);
    }
    harnesses
        .iter()
        .map(|harness| finding_for_harness(harness, verdicts.verdict_for(&harness.full_name)))
        .collect()
}

fn blocked_findings_for_package(
    harnesses: &[KaniHarness],
    run: &PackageRun,
) -> Result<Vec<Finding>, KaniLaneError> {
    let reason = blocked_reason(run);
    harnesses
        .iter()
        .map(|harness| {
            let rule_id = rule_id_for_harness(harness, FALLBACK_RULE_BLOCKED)?;
            Ok(harness_finding(
                rule_id,
                FindingEffect::Reject,
                format!(
                    "package={} harness={} reason={reason}",
                    harness.package, harness.full_name
                ),
                format!(
                    "Kani harness `{}` blocked by cgroup OOM or wallclock timeout.",
                    harness.full_name
                ),
                kani_location(&run.cmd),
            ))
        })
        .collect()
}

fn blocked_reason(run: &PackageRun) -> String {
    if run.timed_out {
        return format!("timed_out_after_{PER_PACKAGE_TIMEOUT_SECS}s");
    }
    let teardown = run.teardown_errors.first().map_or("", |s| s.as_str());
    if teardown.is_empty() {
        String::from("spawn_or_wait_failed")
    } else {
        format!("spawn_or_wait_failed; teardown={teardown}")
    }
}

fn finding_for_harness(
    harness: &KaniHarness,
    verdict: HarnessVerdict,
) -> Result<Finding, KaniLaneError> {
    Ok(match verdict {
        HarnessVerdict::Successful => pass_finding(harness)?,
        HarnessVerdict::Failed => fail_finding(harness, "VERIFICATION:- FAILED")?,
        HarnessVerdict::Unsupported => unsupported_finding(harness)?,
        HarnessVerdict::Unknown => no_verdict_finding(harness)?,
    })
}

fn pass_finding(harness: &KaniHarness) -> Result<Finding, KaniLaneError> {
    let rule_id = rule_id_for_harness(harness, FALLBACK_RULE_PASS)?;
    let label = harness_label(harness);
    Ok(harness_finding(
        rule_id,
        FindingEffect::Informational,
        format!("package={} harness={} verdict=SUCCESSFUL", harness.package, label),
        format!("Kani harness `{label}` in `{}` verified.", harness.package),
        kani_location_for_harness(harness),
    ))
}

fn fail_finding(harness: &KaniHarness, reason: &str) -> Result<Finding, KaniLaneError> {
    let rule_id = rule_id_for_harness(harness, FALLBACK_RULE_FAIL)?;
    let label = harness_label(harness);
    Ok(harness_finding(
        rule_id,
        FindingEffect::Reject,
        format!("package={} harness={} reason={reason}", harness.package, label),
        format!("Investigate Kani harness `{label}` in package `{}`.", harness.package),
        kani_location_for_harness(harness),
    ))
}

fn no_verdict_finding(harness: &KaniHarness) -> Result<Finding, KaniLaneError> {
    let rule_id = rule_id_for_harness(harness, FALLBACK_RULE_NO_VERDICT)?;
    let label = harness_label(harness);
    Ok(harness_finding(
        rule_id,
        FindingEffect::Reject,
        format!("package={} harness={} reason=missing_verification_line", harness.package, label),
        format!(
            "Kani harness `{label}` in `{}` produced no `VERIFICATION:-` line; \
             cargo-kani may have emitted unexpected output.",
            harness.package
        ),
        kani_location_for_harness(harness),
    ))
}

fn unsupported_finding(harness: &KaniHarness) -> Result<Finding, KaniLaneError> {
    let rule_id = rule_id_for_harness(harness, FALLBACK_RULE_UNSUPPORTED)?;
    let label = harness_label(harness);
    Ok(harness_finding(
        rule_id,
        FindingEffect::Informational,
        format!("package={} harness={} verdict=UNSUPPORTED", harness.package, label),
        format!("Kani harness `{label}` disabled by unsupported-feature warning."),
        kani_location_for_harness(harness),
    ))
}

fn infra_finding(reason: &str) -> Result<Finding, KaniLaneError> {
    let rule_id = build_rule_id(FALLBACK_RULE_INFRA, FALLBACK_RULE_FAIL)?;
    Ok(harness_finding(
        rule_id,
        FindingEffect::Reject,
        format!("infra-failure: {reason}"),
        "Kani could not produce a parseable run.".to_owned(),
        Location::workspace(),
    ))
}

fn build_clean_outcome(
    state: &LaneRunState,
    worst: &PackageRun,
) -> Result<LaneOutcome, OutcomeError> {
    let command = CommandEvidence::new(worst.cmd.executable.clone(), worst.cmd.argv.clone())?;
    let exit_status = ProcessTermination::Exited { code: state.worst_exit_code };
    let evidence = LaneEvidence::new(
        command,
        state.tool_version.clone(),
        exit_status,
        Digest::from_bytes(state.tool_version.as_bytes()),
    )?;
    if !worst.is_successful() {
        return Err(OutcomeError::NonZeroExit);
    }
    Ok(LaneOutcome::Clean { evidence })
}

fn kani_location(_cmd: &PackageCmd) -> Location {
    Location::tool(KANI_TOOL.to_owned(), current_kani_version())
}

fn kani_location_for_harness(_harness: &KaniHarness) -> Location {
    Location::tool(KANI_TOOL.to_owned(), current_kani_version())
}

fn build_rule_id(literal: &str, fallback: &str) -> Result<RuleId, KaniLaneError> {
    RuleId::new(literal).map_or_else(|_| RuleId::new(fallback).map_err(KaniLaneError::RuleId), Ok)
}

fn rule_id_for_harness(harness: &KaniHarness, fallback: &str) -> Result<RuleId, KaniLaneError> {
    harness.canonical_id.as_ref().map_or_else(
        || RuleId::new(fallback).map_err(KaniLaneError::RuleId),
        |id| build_rule_id(&format!("PROOF_KANI_{}", id.as_str()), fallback),
    )
}

const fn harness_finding(
    rule_id: RuleId,
    effect: FindingEffect,
    message: String,
    repair: String,
    location: Location,
) -> Finding {
    let repair = RepairHint::requires_human_review(repair);
    match effect {
        FindingEffect::Reject => Finding::reject(Lane::Kani, rule_id, location, message, repair),
        FindingEffect::Informational => {
            Finding::informational(Lane::Kani, rule_id, location, message, repair)
        }
    }
}

fn harness_label(harness: &KaniHarness) -> String {
    harness
        .canonical_id
        .as_ref()
        .map_or_else(|| harness.full_name.clone(), |id| id.as_str().to_owned())
}

fn canonical_harness_id(full_name: &str) -> Option<KaniHarnessId> {
    let prefix = "kani::";
    let stripped: &str = full_name.strip_prefix(prefix).map_or(full_name, |rest| rest);
    let upper: String = stripped
        .chars()
        .map(|c| if c.is_ascii_lowercase() { c.to_ascii_uppercase() } else { c })
        .map(|c| if c.is_ascii_uppercase() || c.is_ascii_digit() { c } else { '_' })
        .collect();
    let Ok(id) = KaniHarnessId::new(&upper) else {
        return None;
    };
    Some(id)
}

fn has_workspace_root(manifest_path: &Path) -> bool {
    let Ok(contents) = std::fs::read_to_string(manifest_path) else {
        return false;
    };
    contents.lines().any(|line| line.trim() == "[workspace]")
}

type PackageGroup = (String, Vec<KaniHarness>);

fn group_by_package(inventory: &[KaniHarness]) -> Vec<PackageGroup> {
    inventory
        .iter()
        .fold(BTreeMap::new(), |mut groups: BTreeMap<String, Vec<KaniHarness>>, harness| {
            groups.entry(harness.package.clone()).or_default().push(harness.clone());
            groups
        })
        .into_iter()
        .collect()
}

type CachedKaniProbe = Result<Option<DetectedCargoKani>, KaniVersionError>;

fn cargo_kani_supported() -> CachedKaniProbe {
    static CACHED: OnceLock<CachedKaniProbe> = OnceLock::new();
    CACHED.get_or_init(detect_cargo_kani).clone()
}

fn current_kani_version() -> String {
    let fallback =
        || format!("{}.{}.{}", MIN_KANI_VERSION.0, MIN_KANI_VERSION.1, MIN_KANI_VERSION.2);
    match cargo_kani_supported() {
        Ok(Some(detected)) => detected.version,
        Ok(None) | Err(_) => fallback(),
    }
}

fn probe_systemd_run() -> bool {
    static AVAILABLE: OnceLock<bool> = OnceLock::new();
    *AVAILABLE.get_or_init(detect_systemd_run)
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

fn detect_cargo_kani() -> Result<Option<DetectedCargoKani>, KaniVersionError> {
    let output = Command::new("cargo")
        .arg("kani")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();
    let Ok(output) = output else {
        return Ok(None);
    };
    if !output.status.success() {
        return Ok(None);
    }
    let Ok(text) = std::str::from_utf8(&output.stdout) else {
        return Err(KaniVersionError::NonUtf8Output);
    };
    parse_kani_version(text)
}

fn parse_kani_version(stdout: &str) -> Result<Option<DetectedCargoKani>, KaniVersionError> {
    let head = stdout.lines().next().ok_or_else(|| unparseable_version(stdout))?;
    let mut tokens = head.split_whitespace();
    let name = tokens.next().ok_or_else(|| unparseable_version(stdout))?;
    if name != "cargo-kani" {
        return Err(unparseable_version(stdout));
    }
    let version_str = tokens.next().ok_or_else(|| unparseable_version(stdout))?;
    let mut parts = version_str.split('.');
    let major = parse_version_part(&mut parts, stdout)?;
    let minor = parse_version_part(&mut parts, stdout)?;
    let patch = parse_version_part(&mut parts, stdout)?;
    let detected = DetectedCargoKani { version: version_str.to_owned(), major, minor, patch };
    Ok(detected.meets_floor().then_some(detected))
}

fn unparseable_version(output: &str) -> KaniVersionError {
    KaniVersionError::UnparseableOutput { output: output.into() }
}

fn parse_version_part<'a>(
    parts: &mut impl Iterator<Item = &'a str>,
    output: &str,
) -> Result<u64, KaniVersionError> {
    let part = parts.next().ok_or_else(|| unparseable_version(output))?;
    let Ok(number) = part.parse() else {
        return Err(unparseable_version(output));
    };
    Ok(number)
}

fn run_package(
    workspace_root: &Path,
    package: &str,
    cgroup_available: bool,
) -> Result<PackageRun, KaniRunError> {
    let cmd = if cgroup_available {
        build_cgroup_command(workspace_root, package)
    } else {
        build_bare_command(workspace_root, package)
    };
    let mut spawned = spawn_compiled(&cmd)?;
    let timeout = Duration::from_secs(PER_PACKAGE_TIMEOUT_SECS);
    let inner = run_child_with_timeout(&mut spawned, timeout, &cmd.executable)?;
    Ok(PackageRun {
        exit_code: inner.exit_code,
        timed_out: inner.timed_out,
        stdout: inner.stdout,
        cmd: PackageCmd { executable: cmd.executable, argv: cmd.argv },
        teardown_errors: Vec::new(),
    })
}

fn run_child_with_timeout(
    child: &mut Child,
    timeout: Duration,
    program: &str,
) -> Result<PackageRun, KaniRunError> {
    match child.wait_timeout(timeout) {
        Ok(Some(status)) => complete_child(child, status, program),
        Ok(None) => terminate_timed_out_child(child, program),
        Err(source) => Err(KaniRunError::WaitTimeout { program: program.to_owned(), source }),
    }
}

fn complete_child(
    child: &mut Child,
    status: ExitStatus,
    program: &str,
) -> Result<PackageRun, KaniRunError> {
    let stdout = drain_pipe(program, "stdout", child.stdout.take())?;
    let stderr = drain_pipe(program, "stderr", child.stderr.take())?;
    let mut combined = stdout;
    combined.push('\n');
    combined.push_str(&stderr);
    Ok(PackageRun {
        exit_code: status.code(),
        timed_out: false,
        stdout: combined,
        cmd: PackageCmd { executable: program.to_owned(), argv: Box::new([]) },
        teardown_errors: Vec::new(),
    })
}

fn terminate_timed_out_child(child: &mut Child, program: &str) -> Result<PackageRun, KaniRunError> {
    if let Err(source) = child.kill() {
        return Err(KaniRunError::Kill { program: program.to_owned(), source });
    }
    if let Err(source) = child.wait() {
        return Err(KaniRunError::WaitReap { program: program.to_owned(), source });
    }
    Ok(PackageRun {
        exit_code: None,
        timed_out: true,
        stdout: String::new(),
        cmd: PackageCmd { executable: program.to_owned(), argv: Box::new([]) },
        teardown_errors: Vec::new(),
    })
}

struct CompiledCommand {
    executable: String,
    argv: Box<[String]>,
    working_dir: PathBuf,
}

fn spawn_compiled(cmd: &CompiledCommand) -> Result<Child, KaniRunError> {
    let mut builder = Command::new(&cmd.executable);
    let _ = builder.args(cmd.argv.iter().skip(1).cloned());
    let _ = builder.current_dir(&cmd.working_dir);
    let _ = builder.stdin(Stdio::null());
    let _ = builder.stdout(Stdio::piped());
    let _ = builder.stderr(Stdio::piped());
    builder
        .spawn()
        .map_err(|source| KaniRunError::Spawn { program: cmd.executable.clone(), source })
}

fn build_cgroup_command(workspace_root: &Path, package: &str) -> CompiledCommand {
    let argv: Box<[String]> = Vec::from([
        String::from("systemd-run"),
        String::from("--user"),
        String::from("--scope"),
        String::from("-p"),
        format!("MemoryMax={CGROUP_MEMORY_MAX}"),
        String::from("-p"),
        String::from("MemorySwapMax=0"),
        String::from("--"),
        String::from("cargo"),
        String::from("kani"),
        String::from("-p"),
        package.to_owned(),
        String::from("--output-format"),
        String::from("regular"),
    ])
    .into_boxed_slice();
    CompiledCommand {
        executable: String::from("systemd-run"),
        argv,
        working_dir: workspace_root.to_path_buf(),
    }
}

fn build_bare_command(workspace_root: &Path, package: &str) -> CompiledCommand {
    let argv: Box<[String]> = Vec::from([
        String::from("cargo"),
        String::from("kani"),
        String::from("-p"),
        package.to_owned(),
        String::from("--output-format"),
        String::from("regular"),
    ])
    .into_boxed_slice();
    CompiledCommand {
        executable: String::from("cargo"),
        argv,
        working_dir: workspace_root.to_path_buf(),
    }
}

#[derive(Default)]
struct InventoryAggregate {
    harnesses: Vec<KaniHarness>,
    any_listed: bool,
    errors: Vec<String>,
}

fn list_kani_harnesses(
    workspace_root: &Path,
    state: &mut LaneRunState,
) -> Result<Vec<KaniHarness>, KaniInventoryError> {
    let crates_dir = workspace_root.join("crates");
    if !crates_dir.exists() {
        return Err(KaniInventoryError::NoCratesDir(crates_dir.display().to_string()));
    }
    let entries = std::fs::read_dir(&crates_dir).map_err(|source| {
        KaniInventoryError::CratesDirRead { path: crates_dir.display().to_string(), source }
    })?;
    let crate_dirs: Vec<(PathBuf, String)> =
        entries.flatten().filter_map(|entry| crate_entry(&entry)).collect();
    let aggregate =
        crate_dirs.iter().fold(InventoryAggregate::default(), |mut aggregate, (path, pkg)| {
            accumulate_inventory(&mut aggregate, pkg, list_kani_for_crate(path, pkg, state));
            aggregate
        });
    if !aggregate.any_listed && aggregate.harnesses.is_empty() && aggregate.errors.is_empty() {
        return Err(KaniInventoryError::NoCratesDir(crates_dir.display().to_string()));
    }
    Ok(aggregate.harnesses)
}

fn crate_entry(entry: &std::fs::DirEntry) -> Option<(PathBuf, String)> {
    let path = entry.path();
    if !path.is_dir() {
        return None;
    }
    if !path.join("Cargo.toml").exists() {
        return None;
    }
    let pkg = entry.file_name().to_string_lossy().into_owned();
    if pkg == "titania-dylint" {
        return None;
    }
    Some((path, pkg))
}

fn accumulate_inventory(
    aggregate: &mut InventoryAggregate,
    pkg: &str,
    result: Result<Vec<KaniHarness>, KaniInventoryError>,
) {
    match result {
        Ok(mut harnesses) => {
            mark_listed(aggregate, &harnesses);
            aggregate.harnesses.append(&mut harnesses);
        }
        Err(error) => aggregate.errors.push(format!("{pkg}: {error}")),
    }
}

const fn mark_listed(aggregate: &mut InventoryAggregate, harnesses: &[KaniHarness]) {
    if !harnesses.is_empty() {
        aggregate.any_listed = true;
    }
}

fn list_kani_for_crate(
    crate_dir: &Path,
    package: &str,
    state: &mut LaneRunState,
) -> Result<Vec<KaniHarness>, KaniInventoryError> {
    let artifact = crate_dir.join("kani-list.json");
    cleanup_list_artifact(&artifact).map_err(|source| KaniInventoryError::CleanupFailed {
        path: artifact.display().to_string(),
        source,
    })?;
    let output = Command::new("cargo")
        .current_dir(crate_dir)
        .arg("kani")
        .arg("list")
        .arg("--format")
        .arg("json")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|source| KaniInventoryError::ListCommand {
            crate_name: package.to_owned(),
            source,
        })?;
    let stderr_text = String::from_utf8_lossy(&output.stderr).into_owned();
    let stderr: Box<str> = stderr_text.into_boxed_str();
    let code = output.status.code();
    if let Some(reported) = code {
        state.worst_exit_code = state.worst_exit_code.max(reported);
    }
    if !output.status.success() {
        return Err(KaniInventoryError::ListCommandExit {
            crate_name: package.to_owned(),
            code,
            stderr,
        });
    }
    let contents = std::fs::read_to_string(&artifact).map_err(|source| {
        KaniInventoryError::ArtifactRead { path: artifact.display().to_string(), source }
    })?;
    parse_kani_list(package, &contents).map_err(|source| KaniInventoryError::ArtifactParse {
        path: artifact.display().to_string(),
        source,
    })
}

fn cleanup_list_artifact(artifact: &Path) -> Result<(), std::io::Error> {
    match std::fs::remove_file(artifact) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[derive(Deserialize)]
struct KaniListFile {
    #[serde(default, rename = "standard-harnesses")]
    standard_harnesses: std::collections::BTreeMap<String, Vec<String>>,
    #[serde(default, rename = "contract-harnesses")]
    contract_harnesses: std::collections::BTreeMap<String, Vec<String>>,
}

fn parse_kani_list(package: &str, contents: &str) -> Result<Vec<KaniHarness>, serde_json::Error> {
    let file: KaniListFile = serde_json::from_str(contents)?;
    let mut all = harnesses_from_file_map(package, &file.standard_harnesses);
    all.extend(harnesses_from_file_map(package, &file.contract_harnesses));
    Ok(all)
}

fn harnesses_from_file_map(
    package: &str,
    file_map: &std::collections::BTreeMap<String, Vec<String>>,
) -> Vec<KaniHarness> {
    file_map
        .values()
        .flat_map(|harnesses| {
            harnesses.iter().map(|name| KaniHarness {
                package: package.to_owned(),
                full_name: name.clone(),
                canonical_id: canonical_harness_id(name),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        DetectedCargoKani, FALLBACK_RULE_BLOCKED, FALLBACK_RULE_INFRA, FALLBACK_RULE_NO_HARNESSES,
        FALLBACK_RULE_NO_VERDICT, HarnessVerdict, HarnessVerdictMap, KaniHarness,
        KaniInventoryError, KaniLaneError, KaniRunError, KaniVersionError, LaneRunState,
        MIN_KANI_VERSION, PER_PACKAGE_TIMEOUT_SECS, PIPE_BYTE_CAP, PackageCmd, PackageRun,
        blocked_findings_for_package, build_clean_outcome, build_no_harness_outcome,
        canonical_harness_id, checking_harness_name, findings_for_package, inventory_error_outcome,
        is_subcommand_missing, parse_kani_version, verdict_from_line,
    };
    use titania_core::{
        FindingEffect, KANI_HARNESS_ID_MAX_LEN, LaneOutcome, Location, RuleIdError, SkipReason,
        ToolKind,
    };

    #[test]
    fn checking_harness_name_strips_trailing_ellipsis() {
        let line = "Checking harness kani::lane_name_rejects_empty_string...";
        assert_eq!(checking_harness_name(line), Some("kani::lane_name_rejects_empty_string"));
    }

    #[test]
    fn checking_harness_name_returns_none_for_unrelated_lines() {
        assert_eq!(checking_harness_name("Compiling titania-core v0.1.0"), None);
        assert_eq!(checking_harness_name(""), None);
    }

    #[test]
    fn verdict_from_line_parses_known_markers() {
        assert_eq!(
            verdict_from_line("VERIFICATION:- SUCCESSFUL"),
            Some(HarnessVerdict::Successful)
        );
        assert_eq!(verdict_from_line("VERIFICATION:- FAILED"), Some(HarnessVerdict::Failed));
        assert_eq!(
            verdict_from_line("VERIFICATION:- UNSUPPORTED (caller_location)"),
            Some(HarnessVerdict::Unsupported)
        );
        assert_eq!(verdict_from_line("Compiling titania-core"), None);
    }

    #[test]
    fn verdict_map_pairs_checking_with_verification_lines() {
        let stdout = "\
Checking harness kani::foo_bar...
some build noise
VERIFICATION:- SUCCESSFUL
Checking harness kani::baz_qux...
VERIFICATION:- FAILED
";
        let map = HarnessVerdictMap::from_stdout(stdout);
        assert_eq!(map.verdict_for("kani::foo_bar"), HarnessVerdict::Successful);
        assert_eq!(map.verdict_for("kani::baz_qux"), HarnessVerdict::Failed);
        assert_eq!(map.verdict_for("kani::never_run"), HarnessVerdict::Unknown);
    }

    #[test]
    fn canonical_harness_id_uppercases_lowercase_names() {
        let id = canonical_harness_id("kani::lane_name_rejects_empty_string")
            .expect("canonical id must be valid");
        assert_eq!(id.as_str(), "LANE_NAME_REJECTS_EMPTY_STRING");
    }

    #[test]
    fn canonical_harness_id_rejects_invalid_form() {
        assert!(canonical_harness_id("").is_none());
        assert!(canonical_harness_id("123_LEADING_DIGIT").is_none());
        let too_long = "X".repeat(KANI_HARNESS_ID_MAX_LEN + 1);
        assert!(canonical_harness_id(&too_long).is_none());
    }

    #[test]
    fn parse_kani_version_accepts_supported_release() {
        let version = parse_kani_version("cargo-kani 0.67.0 (cargo plugin)")
            .expect("0.67.0 must probe cleanly")
            .expect("0.67.0 must meet the floor");
        assert_eq!(version.version, "0.67.0");
        assert_eq!(version.major, 0);
        assert_eq!(version.minor, 67);
        assert_eq!(version.patch, 0);
        assert!(version.meets_floor());
    }

    #[test]
    fn parse_kani_version_rejects_too_old_release() {
        let probe = parse_kani_version("cargo-kani 0.49.3 (cargo plugin)")
            .expect("well-formed output must probe cleanly");
        assert!(probe.is_none());
    }

    #[test]
    fn parse_kani_version_rejects_garbled_output() {
        assert!(matches!(
            parse_kani_version("cargo-kani 0.67"),
            Err(KaniVersionError::UnparseableOutput { .. })
        ));
        assert!(matches!(
            parse_kani_version("cargo-kani x.y.z"),
            Err(KaniVersionError::UnparseableOutput { .. })
        ));
        assert!(matches!(
            parse_kani_version("kani 0.67.0"),
            Err(KaniVersionError::UnparseableOutput { .. })
        ));
        assert!(matches!(parse_kani_version(""), Err(KaniVersionError::UnparseableOutput { .. })));
    }

    #[test]
    fn meets_floor_uses_tuple_ordering() {
        let floor_major = DetectedCargoKani {
            version: format!(
                "{}.{}.{}",
                MIN_KANI_VERSION.0, MIN_KANI_VERSION.1, MIN_KANI_VERSION.2
            ),
            major: MIN_KANI_VERSION.0,
            minor: MIN_KANI_VERSION.1,
            patch: MIN_KANI_VERSION.2,
        };
        assert!(floor_major.meets_floor());
        let below =
            DetectedCargoKani { version: String::from("0.49.99"), major: 0, minor: 49, patch: 99 };
        assert!(!below.meets_floor());
    }

    #[test]
    fn inventory_error_outcome_skips_when_subcommand_missing() {
        let error = KaniInventoryError::ListCommandExit {
            crate_name: String::from("titania-core"),
            code: Some(1),
            stderr: String::from("error: no such subcommand: `kani`").into_boxed_str(),
        };
        let outcome =
            inventory_error_outcome(&error).expect("missing cargo-kani must map to Skipped");
        assert!(matches!(
            outcome,
            LaneOutcome::Skipped { reason: SkipReason::ToolUnavailable(ToolKind::CargoKani) }
        ));
    }

    #[test]
    fn inventory_error_outcome_skips_when_binary_not_found() {
        let error = KaniInventoryError::ListCommand {
            crate_name: String::from("titania-core"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "cargo-kani not found"),
        };
        let outcome =
            inventory_error_outcome(&error).expect("missing cargo-kani binary must map to Skipped");
        assert!(matches!(
            outcome,
            LaneOutcome::Skipped { reason: SkipReason::ToolUnavailable(ToolKind::CargoKani) }
        ));
    }

    #[test]
    fn is_subcommand_missing_recognises_both_phrases() {
        let missing_subcommand = KaniInventoryError::ListCommandExit {
            crate_name: String::from("titania-core"),
            code: Some(1),
            stderr: String::from("no such subcommand").into_boxed_str(),
        };
        let missing_binary = KaniInventoryError::ListCommand {
            crate_name: String::from("titania-core"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "binary not found"),
        };
        let artifact_error = KaniInventoryError::ArtifactRead {
            path: String::from("/tmp/kani-list.json"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        assert!(is_subcommand_missing(&missing_subcommand));
        assert!(is_subcommand_missing(&missing_binary));
        assert!(!is_subcommand_missing(&artifact_error));
    }

    #[test]
    fn inventory_error_outcome_emits_infra_finding_for_other_failures() {
        let error = KaniInventoryError::ArtifactRead {
            path: String::from("/tmp/kani-list.json"),
            source: std::io::Error::new(std::io::ErrorKind::PermissionDenied, "denied"),
        };
        let outcome =
            inventory_error_outcome(&error).expect("infra failures must produce a Finding");
        let LaneOutcome::Findings { findings } = outcome else {
            panic!("infra failures must surface as Findings, not Skipped");
        };
        assert_eq!(findings.len(), 1);
        assert!(findings[0].is_reject());
        assert_eq!(findings[0].rule_id().as_str(), FALLBACK_RULE_INFRA);
    }

    #[test]
    fn no_harness_outcome_is_findings_not_clean() {
        let state = LaneRunState { worst_exit_code: 0, tool_version: String::from("0.67.0") };
        let outcome =
            build_no_harness_outcome(&state).expect("empty inventory must produce a typed outcome");
        match outcome {
            LaneOutcome::Findings { findings } => {
                assert_eq!(findings.len(), 1);
                assert!(findings[0].is_informational());
                assert_eq!(findings[0].rule_id().as_str(), FALLBACK_RULE_NO_HARNESSES);
            }
            other => panic!("empty inventory must not be Clean, got {other:?}"),
        }
    }

    #[test]
    fn timed_out_package_emits_blocked_finding_per_harness() {
        let harnesses = vec![
            harness_fixture("titania-core", "kani::lane_name_rejects_empty_string"),
            harness_fixture("titania-core", "kani::lane_name_rejects_nul_byte"),
        ];
        let run = timed_out_run_fixture();
        let findings = blocked_findings_for_package(&harnesses, &run)
            .expect("blocked-findings must construct");
        assert_eq!(findings.len(), 2);
        for finding in &findings {
            assert!(finding.is_reject());
            assert_eq!(finding.effect(), FindingEffect::Reject);
        }
    }

    #[test]
    fn spawn_failure_package_emits_blocked_findings_with_teardown_context() {
        let harnesses = vec![harness_fixture("titania-core", "kani::some_harness")];
        let mut run = timed_out_run_fixture();
        run.exit_code = None;
        run.timed_out = false;
        run.teardown_errors.push(String::from("wait failed: broken pipe"));
        let findings = blocked_findings_for_package(&harnesses, &run)
            .expect("spawn/wait failures must surface as blocked findings");
        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert!(finding.is_reject());
        assert!(
            finding.message().contains("spawn_or_wait_failed"),
            "blocked finding must mention spawn/wait failure, got: {}",
            finding.message()
        );
    }

    #[test]
    fn verdict_driven_package_emits_one_finding_per_harness() {
        let harnesses = vec![
            harness_fixture("titania-core", "kani::foo_bar"),
            harness_fixture("titania-core", "kani::baz_qux"),
        ];
        let verdicts = HarnessVerdictMap::from_stdout(
            "Checking harness kani::foo_bar...\nVERIFICATION:- SUCCESSFUL\n\
             Checking harness kani::baz_qux...\nVERIFICATION:- FAILED\n",
        );
        let run = completed_run_fixture(0, true);
        let findings = findings_for_package(&harnesses, &verdicts, &run)
            .expect("verdict-driven findings must construct");
        assert_eq!(findings.len(), 2);
        assert!(findings[0].is_informational());
        assert!(findings[1].is_reject());
    }

    #[test]
    fn missing_verdict_distinguishes_unknown_from_blocked() {
        let harnesses = vec![harness_fixture("titania-core", "kani::123-leading-digit")];
        let verdicts = HarnessVerdictMap::default();
        let run = completed_run_fixture(0, true);
        let findings = findings_for_package(&harnesses, &verdicts, &run)
            .expect("missing-verdict findings must construct");
        assert_eq!(findings.len(), 1);
        let finding = &findings[0];
        assert!(finding.is_reject(), "missing verdict must reject, got {finding:?}");
        assert_eq!(
            finding.rule_id().as_str(),
            FALLBACK_RULE_NO_VERDICT,
            "missing verdict must use NO_VERDICT, not BLOCKED"
        );
        assert_ne!(
            finding.rule_id().as_str(),
            FALLBACK_RULE_BLOCKED,
            "missing verdict must NOT use the BLOCKED rule id"
        );
    }

    #[test]
    fn clean_outcome_records_real_cgroup_argv() {
        let state = LaneRunState { worst_exit_code: 0, tool_version: String::from("0.67.0") };
        let run = cgroup_run_fixture();
        let outcome = build_clean_outcome(&state, &run)
            .expect("clean outcome must construct on a successful cgroup run");
        let LaneOutcome::Clean { evidence } = outcome else {
            panic!("clean outcome must wrap LaneEvidence");
        };
        let argv = evidence.command().argv();
        assert_eq!(argv.first().map(String::as_str), Some("systemd-run"));
        assert!(argv.iter().any(|arg| arg == "--user"));
        assert!(argv.iter().any(|arg| arg == "--scope"));
        assert!(argv.iter().any(|arg| arg.contains("MemoryMax=24G")));
        assert!(argv.iter().any(|arg| arg == "MemorySwapMax=0"));
        assert!(argv.iter().any(|arg| arg == "cargo"));
        assert!(argv.iter().any(|arg| arg == "kani"));
        assert!(argv.iter().any(|arg| arg == "--output-format"));
        assert!(argv.iter().any(|arg| arg == "regular"));
        assert_eq!(evidence.command().executable(), "systemd-run");
        assert_eq!(evidence.exit_status().exit_code(), Some(0));
    }

    #[test]
    fn clean_outcome_records_real_bare_argv_when_cgroup_unavailable() {
        let state = LaneRunState { worst_exit_code: 0, tool_version: String::from("0.67.0") };
        let run = bare_run_fixture();
        let outcome = build_clean_outcome(&state, &run)
            .expect("clean outcome must construct on a successful bare run");
        let LaneOutcome::Clean { evidence } = outcome else {
            panic!("clean outcome must wrap LaneEvidence");
        };
        let argv = evidence.command().argv();
        assert_eq!(argv.first().map(String::as_str), Some("cargo"));
        assert!(argv.iter().any(|arg| arg == "kani"));
        assert!(argv.iter().any(|arg| arg == "-p"));
        assert!(argv.iter().any(|arg| arg == "titania-core"));
        assert!(!argv.iter().any(|arg| arg == "systemd-run"));
        assert_eq!(evidence.command().executable(), "cargo");
    }

    #[test]
    fn clean_outcome_rejects_non_zero_worst_exit() {
        let state = LaneRunState { worst_exit_code: 2, tool_version: String::from("0.67.0") };
        let mut run = bare_run_fixture();
        run.exit_code = Some(2);
        let error = build_clean_outcome(&state, &run).expect_err("non-zero worst exit must reject");
        assert!(
            matches!(error, titania_core::OutcomeError::NonZeroExit),
            "non-zero worst exit must produce OutcomeError, got {error:?}"
        );
    }

    #[test]
    fn package_run_successful_classifies_zero_exit() {
        let run = cgroup_run_fixture();
        assert!(run.is_successful(), "cgroup run with exit 0 must classify as successful");
    }

    #[test]
    fn pipe_byte_cap_is_large_but_bounded() {
        assert_eq!(PIPE_BYTE_CAP, 8 * 1024 * 1024);
        assert!(PER_PACKAGE_TIMEOUT_SECS > 0);
    }

    #[test]
    fn kani_run_error_variants_are_distinguishable_via_display() {
        let spawn = KaniRunError::Spawn {
            program: String::from("cargo"),
            source: std::io::Error::new(std::io::ErrorKind::NotFound, "missing"),
        };
        let wait = KaniRunError::WaitTimeout {
            program: String::from("cargo"),
            source: std::io::Error::new(std::io::ErrorKind::Other, "broken"),
        };
        assert_ne!(spawn.to_string(), wait.to_string());
        assert!(spawn.to_string().contains("spawn failed"));
        assert!(wait.to_string().contains("wait_timeout"));
    }

    #[test]
    fn rule_id_error_wraps_via_explicit_constructor() {
        let inner = RuleIdError::Empty;
        let wrapped = KaniLaneError::RuleId(inner);
        assert!(matches!(wrapped, KaniLaneError::RuleId(_)));
    }

    #[test]
    fn location_tool_carries_runtime_detected_version() {
        let location = Location::tool(String::from("cargo-kani"), String::from("0.67.0"));
        let value = serde_json::to_value(&location).expect("location serialization must succeed");
        let obj = value.as_object().expect("location serializes to an object");
        assert!(obj.contains_key("Tool"), "Tool variant must round-trip through serde");
    }

    fn harness_fixture(package: &str, full_name: &str) -> KaniHarness {
        KaniHarness {
            package: package.to_owned(),
            full_name: full_name.to_owned(),
            canonical_id: canonical_harness_id(full_name),
        }
    }

    fn timed_out_run_fixture() -> PackageRun {
        PackageRun {
            exit_code: None,
            timed_out: true,
            stdout: String::new(),
            cmd: cgroup_cmd_fixture(),
            teardown_errors: Vec::new(),
        }
    }

    fn completed_run_fixture(exit_code: i32, cgroup_used: bool) -> PackageRun {
        let cmd = if cgroup_used { cgroup_cmd_fixture() } else { bare_cmd_fixture() };
        PackageRun {
            exit_code: Some(exit_code),
            timed_out: false,
            stdout: String::new(),
            cmd,
            teardown_errors: Vec::new(),
        }
    }

    fn cgroup_run_fixture() -> PackageRun {
        PackageRun {
            exit_code: Some(0),
            timed_out: false,
            stdout: String::new(),
            cmd: cgroup_cmd_fixture(),
            teardown_errors: Vec::new(),
        }
    }

    fn bare_run_fixture() -> PackageRun {
        PackageRun {
            exit_code: Some(0),
            timed_out: false,
            stdout: String::new(),
            cmd: bare_cmd_fixture(),
            teardown_errors: Vec::new(),
        }
    }

    fn cgroup_cmd_fixture() -> PackageCmd {
        PackageCmd {
            executable: String::from("systemd-run"),
            argv: [
                "systemd-run",
                "--user",
                "--scope",
                "-p",
                "MemoryMax=24G",
                "-p",
                "MemorySwapMax=0",
                "--",
                "cargo",
                "kani",
                "-p",
                "titania-core",
                "--output-format",
                "regular",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
        }
    }

    fn bare_cmd_fixture() -> PackageCmd {
        PackageCmd {
            executable: String::from("cargo"),
            argv: ["cargo", "kani", "-p", "titania-core", "--output-format", "regular"]
                .into_iter()
                .map(String::from)
                .collect(),
        }
    }
}

#[cfg(test)]
mod size_tests {
    #[test]
    fn assert_kani_lane_error_under_threshold() {
        assert!(std::mem::size_of::<super::KaniLaneError>() < 64);
        assert!(std::mem::size_of::<super::KaniInventoryError>() < 64);
    }
}
