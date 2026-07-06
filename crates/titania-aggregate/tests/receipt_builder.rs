//! Behavior tests for v1 receipt digest building.
//!
//! Bead: tn-d7l.3

use std::error::Error;

use titania_aggregate::receipt_builder::{build_quality_receipt, compute_evidence_digest};
use titania_core::{
    CommandEvidence, Digest, GateScope, Lane, LaneEvidence, LaneReceipt, ProcessTermination,
    ReceiptDigests,
};

type TestResult = Result<(), Box<dyn Error>>;

#[test]
fn receipt_builder_source_digest_alters_quality_receipt() -> TestResult {
    let left = build_quality_receipt(
        GateScope::Edit,
        digests(b"source-a", b"lock", b"policy", b"toolchain"),
        lane_receipts(),
    )?;
    let right = build_quality_receipt(
        GateScope::Edit,
        digests(b"source-b", b"lock", b"policy", b"toolchain"),
        lane_receipts(),
    )?;

    assert_ne!(left.source_digest(), right.source_digest());
    assert_eq!(left.cargo_lock_digest(), right.cargo_lock_digest());
    assert_eq!(left.policy_digest(), right.policy_digest());
    assert_eq!(left.toolchain_digest(), right.toolchain_digest());
    Ok(())
}

#[test]
fn receipt_builder_cargo_lock_digest_alters_quality_receipt() -> TestResult {
    let left = build_quality_receipt(
        GateScope::Edit,
        digests(b"source", b"lock-a", b"policy", b"toolchain"),
        lane_receipts(),
    )?;
    let right = build_quality_receipt(
        GateScope::Edit,
        digests(b"source", b"lock-b", b"policy", b"toolchain"),
        lane_receipts(),
    )?;

    assert_eq!(left.source_digest(), right.source_digest());
    assert_ne!(left.cargo_lock_digest(), right.cargo_lock_digest());
    assert_eq!(left.policy_digest(), right.policy_digest());
    assert_eq!(left.toolchain_digest(), right.toolchain_digest());
    Ok(())
}

#[test]
fn receipt_builder_policy_digest_alters_quality_receipt() -> TestResult {
    let left = build_quality_receipt(
        GateScope::Edit,
        digests(b"source", b"lock", b"policy-a", b"toolchain"),
        lane_receipts(),
    )?;
    let right = build_quality_receipt(
        GateScope::Edit,
        digests(b"source", b"lock", b"policy-b", b"toolchain"),
        lane_receipts(),
    )?;

    assert_eq!(left.source_digest(), right.source_digest());
    assert_eq!(left.cargo_lock_digest(), right.cargo_lock_digest());
    assert_ne!(left.policy_digest(), right.policy_digest());
    assert_eq!(left.toolchain_digest(), right.toolchain_digest());
    Ok(())
}

#[test]
fn receipt_builder_toolchain_digest_alters_quality_receipt() -> TestResult {
    let left = build_quality_receipt(
        GateScope::Edit,
        digests(b"source", b"lock", b"policy", b"toolchain-a"),
        lane_receipts(),
    )?;
    let right = build_quality_receipt(
        GateScope::Edit,
        digests(b"source", b"lock", b"policy", b"toolchain-b"),
        lane_receipts(),
    )?;

    assert_eq!(left.source_digest(), right.source_digest());
    assert_eq!(left.cargo_lock_digest(), right.cargo_lock_digest());
    assert_eq!(left.policy_digest(), right.policy_digest());
    assert_ne!(left.toolchain_digest(), right.toolchain_digest());
    Ok(())
}

#[test]
fn receipt_builder_preserves_lane_receipts() -> TestResult {
    let lanes = vec![
        LaneReceipt::new(Lane::Fmt, digest(b"fmt-evidence"), true),
        LaneReceipt::new(Lane::Compile, digest(b"compile-evidence"), true),
    ]
    .into_boxed_slice();

    let receipt = build_quality_receipt(
        GateScope::Edit,
        digests(b"source", b"lock", b"policy", b"toolchain"),
        lanes.clone(),
    )?;

    assert_eq!(receipt.lanes(), lanes.as_ref());
    Ok(())
}

#[test]
fn receipt_builder_evidence_digest_computed_from_parsed_evidence() -> TestResult {
    let evidence = evidence("cargo", &["cargo", "fmt", "--check"], "cargo 1.90", b"fmt-clean")?;
    let expected_json = serde_json::to_vec(&evidence)?;
    let expected = Digest::from_bytes(&expected_json);

    let actual = compute_evidence_digest(&evidence)?;

    assert_eq!(actual, expected);
    Ok(())
}

#[test]
fn receipt_builder_different_evidence_produces_different_digests() -> TestResult {
    let left = evidence("cargo", &["cargo", "fmt", "--check"], "cargo 1.90", b"fmt-clean")?;
    let right = evidence("cargo", &["cargo", "clippy"], "cargo 1.90", b"clippy-clean")?;

    let left_digest = compute_evidence_digest(&left)?;
    let right_digest = compute_evidence_digest(&right)?;

    assert_ne!(left_digest, right_digest);
    Ok(())
}

#[test]
fn receipt_builder_same_evidence_produces_same_digest() -> TestResult {
    let left = evidence("cargo", &["cargo", "test"], "cargo 1.90", b"tests-clean")?;
    let right = evidence("cargo", &["cargo", "test"], "cargo 1.90", b"tests-clean")?;

    let left_digest = compute_evidence_digest(&left)?;
    let right_digest = compute_evidence_digest(&right)?;

    assert_eq!(left_digest, right_digest);
    Ok(())
}

fn digests(source: &[u8], lock: &[u8], policy: &[u8], toolchain: &[u8]) -> ReceiptDigests {
    ReceiptDigests::new(digest(source), digest(lock), digest(policy), digest(toolchain))
}

fn lane_receipts() -> Box<[LaneReceipt]> {
    Box::new([LaneReceipt::new(Lane::Fmt, digest(b"fmt-evidence"), true)])
}

fn evidence(
    executable: &str,
    argv: &[&str],
    tool_version: &str,
    parsed_digest_seed: &[u8],
) -> Result<LaneEvidence, titania_core::OutcomeError> {
    let argv = argv.iter().map(|arg| (*arg).to_owned()).collect::<Vec<_>>().into_boxed_slice();
    let command = CommandEvidence::new(executable.to_owned(), argv)?;
    LaneEvidence::new(
        command,
        tool_version.to_owned(),
        ProcessTermination::Exited { code: 0 },
        digest(parsed_digest_seed),
    )
}

fn digest(seed: &[u8]) -> Digest {
    Digest::from_bytes(seed)
}
