//! Behavior tests for authoritative proof-registry completeness (bead `tn-f7ti`).
//!
//! The inventory audits `.rs` and `.vr` proof sources against the required
//! obligation registry: a registered missing, unreadable, proof-free, or
//! smoke-only obligation fails; a removed required row fails; a discovered
//! unregistered production proof requires disposition. Coverage integrity
//! only — never a claim of executable correctness.
//!
//! Acceptance command:
//!
//! ```bash
//! cargo test -p titania-lanes --test proof_registry
//! ```

use std::collections::HashMap;

use titania_lanes::proof_registry::{
    AuditFailure, DiscoveredSource, ReadError, ReadErrorKind, Registry, RegistryAuthority,
    RegistryEntry, audit, audit_workspace, classify_source, parse_registry,
};

const RECEIPT_PATH: &str = "verification/verus/receipt_schema.rs";
const SMOKE_PATH: &str = "verification/verus/formal_setup_smoke.rs";

const GENUINE_RECEIPT: &str = r#"use vstd::prelude::*;

#[path = "../../crates/titania-core/src/receipt/schema.rs"]
mod production;

verus! {

pub assume_specification[production::is_supported_receipt_schema_version](schema_version: u32) -> (supported: bool)
    ensures
        supported == (schema_version == 2),
;

proof fn schema_version_literal_is_stable()
    ensures
        2 == 2,
{
    assert(2 == 2);
}

}

fn main() {}
"#;

const SMOKE_FIXTURE: &str = r#"// titania-verus-binding: fixture-smoke
use vstd::prelude::*;

verus! {

proof fn formal_setup_smoke()
    ensures
        1 + 1 == 2,
{
    assert(1 + 1 == 2);
}

}

fn main() {}
"#;

const PROOF_FREE: &str = "fn main() {}\n";

fn production_entry(path: &str) -> RegistryEntry {
    RegistryEntry { path: path.to_owned(), authority: RegistryAuthority::Production }
}

fn fixture_entry(path: &str) -> RegistryEntry {
    RegistryEntry { path: path.to_owned(), authority: RegistryAuthority::SmokeFixture }
}

fn registry_with(entries: Vec<RegistryEntry>) -> Registry {
    Registry { version: 1, targets: entries }
}

fn mem_reader(files: &HashMap<String, String>) -> impl Fn(&str) -> Result<String, ReadError> + '_ {
    move |path: &str| match files.get(path) {
        Some(content) => Ok(content.clone()),
        None => Err(ReadError { path: path.to_owned(), kind: ReadErrorKind::NotFound }),
    }
}

fn two_file_store() -> HashMap<String, String> {
    HashMap::from([
        (RECEIPT_PATH.to_owned(), GENUINE_RECEIPT.to_owned()),
        (SMOKE_PATH.to_owned(), SMOKE_FIXTURE.to_owned()),
    ])
}

// ── happy paths ──────────────────────────────────────────────────────────

/// Registered non-smoke proofs in `.rs` are accounted for by stable identity,
/// and the explicitly non-authoritative smoke fixture is accounted for as a
/// fixture without satisfying production coverage.
#[test]
fn registered_proofs_are_accounted_for_by_stable_identity() {
    let files = two_file_store();
    let registry = registry_with(vec![production_entry(RECEIPT_PATH), fixture_entry(SMOKE_PATH)]);
    let report = audit(mem_reader(&files), &registry, &[]);

    assert!(report.is_clean(), "unexpected failures: {:?}", report.failures());
    assert_eq!(report.passed(), &[RECEIPT_PATH.to_owned(), SMOKE_PATH.to_owned()]);
}

/// Genuine `.vr` content classifies as proof by content, never by extension.
#[test]
fn genuine_vr_content_counts_as_proof_by_content() {
    let body = "proof fn ledger_integrity()\n    ensures\n        1 + 1 == 2,\n{\n    assert(1 + 1 == 2);\n}\n";
    assert_eq!(classify_source(body), titania_lanes::proof_registry::SourceClass::GenuineProof);
}

// ── error paths ──────────────────────────────────────────────────────────

/// The smoke fixture registered as a production obligation fails loudly: a
/// smoke-only file can never satisfy a production obligation.
#[test]
fn smoke_fixture_cannot_satisfy_a_production_obligation() {
    let files = two_file_store();
    let registry =
        registry_with(vec![production_entry(RECEIPT_PATH), production_entry(SMOKE_PATH)]);
    let report = audit(mem_reader(&files), &registry, &[]);

    assert!(!report.is_clean());
    assert_eq!(
        report.failures(),
        &[AuditFailure::RegisteredSmokeAsProduction { obligation: SMOKE_PATH.to_owned() }]
    );
}

/// A registered proof file that was deleted fails with the named obligation.
#[test]
fn deleted_registered_file_fails_the_named_obligation() {
    let files = HashMap::from([(SMOKE_PATH.to_owned(), SMOKE_FIXTURE.to_owned())]);
    let registry = registry_with(vec![production_entry(RECEIPT_PATH), fixture_entry(SMOKE_PATH)]);
    let report = audit(mem_reader(&files), &registry, &[]);

    assert!(!report.is_clean());
    assert!(report.failures().iter().any(|failure| *failure
        == AuditFailure::RegisteredMissing { obligation: RECEIPT_PATH.to_owned() }));
}

/// A registered proof file replaced with proof-free content fails.
#[test]
fn proof_free_replacement_fails_the_named_obligation() {
    let files = HashMap::from([
        (RECEIPT_PATH.to_owned(), PROOF_FREE.to_owned()),
        (SMOKE_PATH.to_owned(), SMOKE_FIXTURE.to_owned()),
    ]);
    let registry = registry_with(vec![production_entry(RECEIPT_PATH), fixture_entry(SMOKE_PATH)]);
    let report = audit(mem_reader(&files), &registry, &[]);

    assert!(!report.is_clean());
    assert_eq!(
        report.failures(),
        &[AuditFailure::RegisteredProofFree { obligation: RECEIPT_PATH.to_owned() }]
    );
}

/// A removed registry row that hides a required obligation blocks, even when
/// every remaining registered file is healthy.
#[test]
fn removed_registry_row_hiding_a_required_obligation_blocks() {
    let files = two_file_store();
    let registry = registry_with(vec![fixture_entry(SMOKE_PATH)]);
    let report = audit(mem_reader(&files), &registry, &[]);

    assert!(!report.is_clean());
    assert!(report.failures().iter().any(|failure| *failure
        == AuditFailure::RequiredObligationRemoved { obligation: RECEIPT_PATH.to_owned() }));
}

/// A discovered unregistered production proof requires disposition instead of
/// passing silently.
#[test]
fn unregistered_production_proof_requires_disposition() {
    let registry = registry_with(vec![production_entry(RECEIPT_PATH), fixture_entry(SMOKE_PATH)]);
    let discovered = [DiscoveredSource {
        path: "verification/verus/unregistered_ledger.vr".to_owned(),
        production: true,
    }];
    let files = HashMap::from([
        (RECEIPT_PATH.to_owned(), GENUINE_RECEIPT.to_owned()),
        (SMOKE_PATH.to_owned(), SMOKE_FIXTURE.to_owned()),
        (
            "verification/verus/unregistered_ledger.vr".to_owned(),
            "proof fn hidden_ledger()\n    ensures\n        3 == 3,\n{\n    assert(3 == 3);\n}\n"
                .to_owned(),
        ),
    ]);
    let report = audit(mem_reader(&files), &registry, &discovered);

    assert!(!report.is_clean());
    assert_eq!(
        report.failures(),
        &[AuditFailure::UnregisteredProductionProof {
            path: "verification/verus/unregistered_ledger.vr".to_owned()
        }]
    );
}

// ── edge cases ───────────────────────────────────────────────────────────

/// Proof words inside comments or string literals are not proof. Only real
/// code tokens count.
#[test]
fn comment_and_string_mentions_are_not_proof() {
    let body = "// proof fn ensures requires invariant assert\nfn main() {\n    let note = \"assume_specification verus kani proof ensures\";\n}\n";
    assert_eq!(classify_source(body), titania_lanes::proof_registry::SourceClass::ProofFree);
}

/// File extension alone is never evidence of proof: empty and code-only
/// bodies classify as proof-free however they are named.
#[test]
fn extension_alone_is_never_evidence_of_proof() {
    assert_eq!(classify_source(""), titania_lanes::proof_registry::SourceClass::ProofFree);
    assert_eq!(classify_source(PROOF_FREE), titania_lanes::proof_registry::SourceClass::ProofFree);
}

/// The explicit smoke tag wins over proof tokens: tagged content is smoke,
/// never production evidence.
#[test]
fn explicit_smoke_tag_wins_over_proof_tokens() {
    assert_eq!(
        classify_source(SMOKE_FIXTURE),
        titania_lanes::proof_registry::SourceClass::SmokeOnly
    );
    assert_eq!(
        classify_source(GENUINE_RECEIPT),
        titania_lanes::proof_registry::SourceClass::GenuineProof
    );
}

// ── registry boundary ────────────────────────────────────────────────────

/// The checked-in registry parses and marks the setup fixture as explicitly
/// non-authoritative.
#[test]
fn checked_in_registry_parses_with_smoke_authority() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let path = std::path::PathBuf::from(manifest).join("../../contracts/proof_obligations.yaml");
    let content = std::fs::read_to_string(&path).expect("read checked-in registry");
    let registry = parse_registry(&content).expect("checked-in registry parses");

    assert_eq!(registry.version, 1);
    let smoke = registry
        .targets
        .iter()
        .find(|entry| entry.path == SMOKE_PATH)
        .expect("smoke fixture stays registered");
    assert_eq!(smoke.authority, RegistryAuthority::SmokeFixture);
    let receipt = registry
        .targets
        .iter()
        .find(|entry| entry.path == RECEIPT_PATH)
        .expect("receipt proof stays registered");
    assert_eq!(receipt.authority, RegistryAuthority::Production);
}

/// Unknown registry fields fail closed instead of being ignored.
#[test]
fn unknown_registry_fields_fail_closed() {
    let content =
        "version: 1\ntargets:\n  - path: verification/verus/receipt_schema.rs\n    kind: proof\n";
    assert!(parse_registry(content).is_err());
}

// ── live inventory ───────────────────────────────────────────────────────

/// The real inventory over the working tree is clean: every registered
/// obligation resolves to proof-bearing content with the declared authority.
#[test]
fn live_workspace_inventory_is_clean() {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    let root = std::path::PathBuf::from(manifest).join("../../");
    let report = audit_workspace(&root).expect("live inventory runs");

    assert!(report.is_clean(), "live inventory failures: {:?}", report.failures());
}
