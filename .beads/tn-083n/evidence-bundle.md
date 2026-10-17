# Evidence bundle — tn-083n

Requirement → source → test → raw command:

1. Verified body names binding + trusted base →
   `trust_binding/{audit,decide}.rs` (`VerifiedProduction{banding,trusted_base}`) →
   `verified_body_with_matching_evidence_names_binding_and_base` →
   `cargo test -p titania-lanes --all-features` (366/366 green).
2. Path-linked assume_spec is source-linked, never proven →
   `decide.rs::audit_one` (evidence stage unreachable without clean markers;
   `SourceLinked` terminal) →
   `path_linked_assume_spec_is_source_linked_never_verified` →
   same suite. Fixture mirrors real `verification/verus/receipt_schema.rs:3-12,27-31`
   (corroborated by grep: `#[path` L3, `assume_specification` L8,
   `production::` L17/24, `2 == 2` ×2).
3. Unbound model / drift blocked →
   `decide.rs` (`MissingBinding`, `MirrorDrift` + `TRUST_MISSING_BINDING`,
   `TRUST_MIRROR_DRIFT`) →
   `unbound_copied_model_is_missing_binding`,
   `changed_body_with_stale_evidence_is_mirror_drift` → same suite.
4. Unapproved markers fail; waivers stay source-linked →
   `decide.rs::uncovered_markers` + `Waiver::is_explicit` →
   `each_trust_marker_without_waiver_blocks` (4 markers × polarity),
   `non_explicit_waiver_covers_nothing` → same suite.
5. Tautology discharges nothing →
   `decide.rs` (`VacuousProof` on zero `production::` refs) →
   `tautology_only_bound_file_is_vacuous` → same suite.
6. Smoke kept separate →
   `decide.rs` (`SmokeOnly` first in decision order, before evidence) →
   `smoke_fixture_never_satisfies_obligation` (passing evidence supplied and
   still blocked) → same suite. Fixture mirrors real
   `verification/verus/formal_setup_smoke.rs:1` (`fixture-smoke`).

Gates (all exit 0 unless noted): `cargo fmt --all -- --check`; `cargo check
--workspace --all-targets --all-features`; source-only strict clippy per
AGENTS.md (33 `-D` flags); `cargo test -p titania-lanes --all-features`
(366 passed / 0 failed); `RUSTDOCFLAGS='-D warnings' cargo doc --workspace
--all-features --no-deps`. `tools/go-skill-v9-validate`: ABSENT (blocker
recorded). `moon ci`: NOT RUN (out of scope, tn-4isu/tn-7bq2).

Landing: no commit/push/sync (not authorized). Bead close left to
parent/orchestrator.
