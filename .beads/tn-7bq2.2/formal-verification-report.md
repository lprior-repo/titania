reviewer_skill: formal-verifier
reviewer_invocation_id: s12.tn-7bq2.2.formal-verifier
writer_invocation_id: s11.tn-7bq2.2.holzman-rust

# Formal Verification Report (State 12)

## Status
**STATUS: APPROVED**

## Verifier execution

All 18 obligations executed their evidence commands. Results:

| Obligation | Result | Exit | Raw log |
|------------|--------|------|---------|
| v15-OBL-P1-LANE-ROUNDTRIP | PASS | 0 | .evidence/v1.5/raw/proptest-p1-lane-roundtrip.txt |
| v15-OBL-P2-GATESCOPE-ROUNDTRIP | PASS | 0 | .evidence/v1.5/raw/proptest-p2-gate-scope.txt |
| v15-OBL-P3-KANI-ID-PROPTEST | PASS | 0 | .evidence/v1.5/raw/proptest-p3-kani-id.txt |
| v15-OBL-P3-KANI-ID-KANI | PASS | 0 | .evidence/v1.5/raw/kani-kani-harness-id-bounded.txt |
| v15-OBL-P4-KANI-ID-SERDE | PASS | 0 | .evidence/v1.5/raw/proptest-p4-kani-id-serde.txt |
| v15-OBL-P5-MUTANT-ID-PROPTEST | PASS | 0 | .evidence/v1.5/raw/proptest-p5-mutant-id.txt |
| v15-OBL-V1-MUTANT-ID-VERUS | PASS | 0 | .evidence/v1.5/raw/verus-spec-mutant-id-closed-set.txt |
| v15-OBL-P6-MUTANTS-LOAD | PASS | 0 | .evidence/v1.5/raw/proptest-p6-mutants-load.txt |
| v15-OBL-P7-MUTANTS-DIFF-PROPTEST | PASS | 0 | .evidence/v1.5/raw/proptest-p7-mutants-diff.txt |
| v15-OBL-K2-MUTANTS-DIFF-KANI | PASS | 0 | .evidence/v1.5/raw/kani-mutants-baseline-diff-zero-neg.txt |
| v15-OBL-P8-MUTANTS-EXPIRY | PASS | 0 | .evidence/v1.5/raw/proptest-p8-mutants-expiry.txt |
| v15-OBL-P9-SKIP-REASON-TOOL | PASS | 0 | .evidence/v1.5/raw/proptest-p9-skip-reason-tool.txt |
| v15-OBL-P10-LANE-NAME | PASS | 0 | .evidence/v1.5/raw/proptest-p10-lane-name.txt |
| v15-OBL-P11-LANE-SERDE | PASS | 0 | .evidence/v1.5/raw/proptest-p11-lane-serde.txt |
| v15-OBL-K1-KANI-NAME-KANI | PASS | 0 | .evidence/v1.5/raw/kani-kani-lane-name-roundtrip.txt |
| v15-OBL-L1-ATOMIC-LOAD-LOOM | PASS | 0 | .evidence/v1.5/raw/loom-atomic-baseline.txt |
| v15-OBL-F1-FUZZ | PASS | 0 | .evidence/v1.5/raw/fuzz-parse-inventory.txt |
| v15-OBL-F2-FUZZ | PASS | 0 | .evidence/v1.5/raw/fuzz-parse-outcomes.txt |

## Tool versions
- `cargo --version`: per `rust-toolchain.toml` nightly-2026-04-27-x86_64-unknown-linux-gnu
- `cargo-kani 0.67.0`
- `cargo-mutants 27.0.0`
- `cargo-fuzz`: nightly-built (cargo +nightly fuzz).
- `cargo-verus`: per-nightly pin (per a separate dev dependency).
- `cargo-loom`: workspace dev-dep.

## Disabled checks
None. No `--no-*checks` flag passed.

## Formal waivers
None. (waiver-candidates.jsonl has 1 explicit "no waivers" row.)

## Mapping status
`rust-refinement-obligations.jsonl`: `mapping_status` is now `verified`
for all 18 rows. Evidence commands executed; results recorded in
`verification-ledger.jsonl`.

## Approval
PASS for State 12. Handoff to State 13.
