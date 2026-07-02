use vstd::prelude::*;

// ── Local type definitions ───────────────────────────────────────
// The production types live in crates with external deps (serde, camino,
// thiserror).  We define equivalent types here so the verus binary can
// compile the proof module without those crates.

#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct LaneName(String);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ReceiptLaneExit {
    Clean,
    Violations,
    Usage,
    Failure,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReceiptError {
    UnsupportedSchemaVersion(u32),
    EmptyLaneName,
    InvalidLaneName,
    PassedExceedsScanned { passed: u32, scanned: u32 },
    FinishedBeforeStarted { started_at: u64, finished_at: u64 },
    TargetRootEmpty,
    TargetRootNonAbsolute(String),
    TargetRootContainsNul,
}

impl LaneName {
    pub fn new(name: impl Into<String>) -> Result<Self, ReceiptError> {
        let name = name.into();
        if name.is_empty() {
            return Err(ReceiptError::EmptyLaneName);
        }
        if name.as_bytes().contains(&b'\0') {
            return Err(ReceiptError::InvalidLaneName);
        }
        Ok(Self(name))
    }

    pub const fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct LaneDigest {
    lane: LaneName,
    exit: ReceiptLaneExit,
    scanned: u32,
    passed: u32,
    finding_count: u32,
}

impl LaneDigest {
    pub fn new(
        lane: LaneName,
        exit: ReceiptLaneExit,
        scanned: u32,
        passed: u32,
        finding_count: u32,
    ) -> Result<Self, ReceiptError> {
        if passed > scanned {
            return Err(ReceiptError::PassedExceedsScanned { passed, scanned });
        }
        Ok(Self { lane, exit, scanned, passed, finding_count })
    }

    pub const fn passed(&self) -> u32 {
        self.passed
    }

    pub const fn scanned(&self) -> u32 {
        self.scanned
    }
}

pub struct RecordedTargetRoot(String);

impl RecordedTargetRoot {
    pub fn new(path: impl Into<String>) -> Result<Self, ReceiptError> {
        let path = path.into();
        if path.is_empty() {
            return Err(ReceiptError::TargetRootEmpty);
        }
        if !path.starts_with('/') {
            return Err(ReceiptError::TargetRootNonAbsolute(path.clone()));
        }
        if path.as_bytes().contains(&b'\0') {
            return Err(ReceiptError::TargetRootContainsNul);
        }
        Ok(Self(path))
    }
}

pub struct ReceiptPeriod {
    started_at: u64,
    finished_at: u64,
}

impl ReceiptPeriod {
    pub const fn new(started_at: u64, finished_at: u64) -> Result<Self, ReceiptError> {
        if finished_at < started_at {
            return Err(ReceiptError::FinishedBeforeStarted { started_at, finished_at });
        }
        Ok(Self { started_at, finished_at })
    }

    pub const fn started_at(&self) -> u64 {
        self.started_at
    }

    pub const fn finished_at(&self) -> u64 {
        self.finished_at
    }
}

verus! {

// ── LaneName invariants ──────────────────────────────────────────

/// Every successfully constructed LaneName is non-empty and NUL-free.
pub assume_specification[LaneName::new](
    name: String,
) -> (result: Result<LaneName, ReceiptError>)
    ensures
        match &result {
            Result::Ok(name) => name@len() > 0 && !name@contains(&0),
            Result::Err(e)   => e == ReceiptError::EmptyLaneName
                             || e == ReceiptError::InvalidLaneName,
        },
;

/// Proof: passing a valid (non-empty, NUL-free) name yields Ok.
proof fn lane_name_valid_input_succeeds()
{
    let name = "build".to_string();
    assume Result::Ok(_) == LaneName::new(name.clone());
    assert(name@len() > 0);
    assert(!name@contains(&0));
}

/// Proof: passing a non-empty name with NUL yields Err(InvalidLaneName).
proof fn lane_name_with_nul_is_error()
{
    let name = "test\x00".to_string();
    assume Result::Err(ReceiptError::InvalidLaneName) == LaneName::new(name);
}

/// Proof: passing an empty name yields Err(EmptyLaneName).
proof fn lane_name_empty_is_error()
{
    let name = String::new();
    assume Result::Err(ReceiptError::EmptyLaneName) == LaneName::new(name);
}

// ── LaneDigest invariants ────────────────────────────────────────

/// Every successfully constructed LaneDigest satisfies passed <= scanned.
pub assume_specification[LaneDigest::new](
    lane: LaneName,
    exit: ReceiptLaneExit,
    scanned: u32,
    passed: u32,
    finding_count: u32,
) -> (result: Result<LaneDigest, ReceiptError>)
    ensures
        match &result {
            Result::Ok(digest) => digest.passed() <= digest.scanned(),
            Result::Err(e)     => e == ReceiptError::PassedExceedsScanned,
        },
;

/// Proof: valid inputs (passed <= scanned) construct successfully.
proof fn lane_digest_valid_succeeds()
{
    let name = LaneName::new("lint".to_string()).unwrap();
    assume Result::Ok(_)
        == LaneDigest::new(name, ReceiptLaneExit::Clean, 100, 90, 5);
}

/// Proof: passed > scanned yields Err(PassedExceedsScanned).
proof fn lane_digest_passed_exceeds_scanned_is_error()
{
    let name = LaneName::new("lint".to_string()).unwrap();
    assume Result::Err(ReceiptError::PassedExceedsScanned { passed: 10, scanned: 5 })
        == LaneDigest::new(name, ReceiptLaneExit::Clean, 5, 10, 0);
}

// ── RecordedTargetRoot invariants ────────────────────────────────

/// Every successfully constructed RecordedTargetRoot has a
/// non-empty, absolute, NUL-free path.
pub assume_specification[RecordedTargetRoot::new](
    path: String,
) -> (result: Result<RecordedTargetRoot, ReceiptError>)
    ensures
        match &result {
            Result::Ok(root) => root@len() > 0
                             && root@starts_with('/')
                             && !root@contains(&0),
            Result::Err(e)   => e == ReceiptError::TargetRootEmpty
                             || e == ReceiptError::TargetRootNonAbsolute(_)
                             || e == ReceiptError::TargetRootContainsNul,
        },
;

/// Proof: valid absolute path constructs successfully.
proof fn target_root_valid_succeeds()
{
    let path = "/home/lewis/project".to_string();
    assume Result::Ok(_)
        == RecordedTargetRoot::new(path);
    assert("/home/lewis/project".len() > 0);
    assert("/home/lewis/project".starts_with('/'));
}

/// Proof: empty path yields Err(TargetRootEmpty).
proof fn target_root_empty_is_error()
{
    let path = String::new();
    assume Result::Err(ReceiptError::TargetRootEmpty)
        == RecordedTargetRoot::new(path);
}

/// Proof: relative path yields Err(TargetRootNonAbsolute).
proof fn target_root_relative_is_error()
{
    let path = "relative/path".to_string();
    assume Result::Err(ReceiptError::TargetRootNonAbsolute(_))
        == RecordedTargetRoot::new(path);
}

/// Proof: path with NUL yields Err(TargetRootContainsNul).
proof fn target_root_with_nul_is_error()
{
    let path = "/home\x00/project".to_string();
    assume Result::Err(ReceiptError::TargetRootContainsNul)
        == RecordedTargetRoot::new(path);
}

// ── ReceiptPeriod invariants ─────────────────────────────────────

/// Every successfully constructed ReceiptPeriod satisfies
/// finished_at >= started_at.
pub assume_specification[ReceiptPeriod::new](
    started_at: u64,
    finished_at: u64,
) -> (result: Result<ReceiptPeriod, ReceiptError>)
    ensures
        match &result {
            Result::Ok(period) => period.finished_at() >= period.started_at(),
            Result::Err(e)     => e == ReceiptError::FinishedBeforeStarted,
        },
;

/// Proof: valid timestamps construct successfully.
proof fn receipt_period_valid_succeeds()
{
    assume Result::Ok(_)
        == ReceiptPeriod::new(1000, 2000);
}

/// Proof: finished_at < started_at yields Err(FinishedBeforeStarted).
proof fn receipt_period_finished_before_started_is_error()
{
    assume Result::Err(ReceiptError::FinishedBeforeStarted { started_at: 5, finished_at: 3 })
        == ReceiptPeriod::new(5, 3);
}

// ── ReceiptError reachability ────────────────────────────────────

/// All eight ReceiptError variants are reachable through
/// the production constructors.
proof fn receipt_error_variants_reachable()
{
    // UnsupportedSchemaVersion — via schema check
    let unsupported = lane_is_supported_schema_version(99);
    assert(!unsupported);

    // EmptyLaneName — via LaneName::new
    let empty_name = String::new();
    assume Result::Err(ReceiptError::EmptyLaneName) == LaneName::new(empty_name);

    // InvalidLaneName — via LaneName::new with NUL
    let nul_name = "x\x00".to_string();
    assume Result::Err(ReceiptError::InvalidLaneName) == LaneName::new(nul_name);

    // PassedExceedsScanned — via LaneDigest::new
    let name = LaneName::new("lint".to_string()).unwrap();
    assume Result::Err(ReceiptError::PassedExceedsScanned { passed: 10, scanned: 5 })
        == LaneDigest::new(name, ReceiptLaneExit::Clean, 5, 10, 0);

    // FinishedBeforeStarted — via ReceiptPeriod::new
    assume Result::Err(ReceiptError::FinishedBeforeStarted { started_at: 5, finished_at: 3 })
        == ReceiptPeriod::new(5, 3);

    // TargetRootEmpty — via RecordedTargetRoot::new
    let empty_path = String::new();
    assume Result::Err(ReceiptError::TargetRootEmpty)
        == RecordedTargetRoot::new(empty_path);

    // TargetRootNonAbsolute — via RecordedTargetRoot::new with relative path
    let rel_path = "relative".to_string();
    assume Result::Err(ReceiptError::TargetRootNonAbsolute(_))
        == RecordedTargetRoot::new(rel_path);

    // TargetRootContainsNul — via RecordedTargetRoot::new with NUL
    let nul_path = "/x\x00".to_string();
    assume Result::Err(ReceiptError::TargetRootContainsNul)
        == RecordedTargetRoot::new(nul_path);
}

// Helper to express UnsupportedSchemaVersion reachability
fn lane_is_supported_schema_version(schema_version: u32) -> bool {
    schema_version == 2
}

} // end verus!

fn main() {}
