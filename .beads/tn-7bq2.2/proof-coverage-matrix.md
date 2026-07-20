# Proof Coverage Matrix

| Req-ID | Contract Clause | Seed | proptest | kani | verus | loom | cargo-fuzz |
|--------|-----------------|------|----------|------|-------|------|------------|
| v15-REQ-LANE-ROUNDTRIP | 3.1 Lane round-trip | v15.P1 | ✓ (P1) | - | - | - | - |
| v15-REQ-GATESCOPE-ROUNDTRIP | 3.2 GateScope round-trip | v15.P2 | ✓ (P2) | - | - | - | - |
| v15-REQ-KANI-HARNESS-ID | 3.3 KaniHarnessId validation | v15.P3 | ✓ (P3-proptest) | — (P3-kani NOT_VERIFIED: cargo-kani 0.67.0 not on PATH; raw log is metadata stub) | - | - | - |
| v15-REQ-KANI-HARNESS-SERDE | 3.3 KaniHarnessId serde | v15.P4 | ✓ (P4) | - | - | - | - |
| v15-REQ-MUTANT-ID | 3.4 MutantId invariants | v15.P5 | ✓ (P5-proptest) | - | - | - | - |
| v15-REQ-MUTANT-ID-OPERATOR | 3.4 operator closed-set | v15.V1 | - | - | — (V1-verus NOT_VERIFIED: spec_mutant_id_closed_set hallucinated symbol, does not exist in crates/titania-core/src/) | - | - |
| v15-REQ-MUTANTS-BASELINE-LOAD | 5 load errors | v15.P6 | ✓ (P6) | - | - | - | - |
| v15-REQ-MUTANTS-DIFF | 4.3 set difference | v15.P7 | ✓ (P7-proptest) | - | - | - | - |
| v15-REQ-MUTANTS-DIFF-ZERO-NEG | 4.3 zero-negatives | v15.K2 | - | — (K2-kani NOT_VERIFIED: cargo-kani not on PATH; raw log is metadata stub) | - | - | - |
| v15-REQ-MUTANTS-EXPIRY | 12-H3 expired | v15.P8 | ✓ (P8) | - | - | - | - |
| v15-REQ-SKIP-REASON-TOOL | 7 skip_state | v15.P9 | ✓ (P9) | - | - | - | - |
| v15-REQ-LANE-NAME | 3.1 name uniqueness | v15.P10 | ✓ (P10) | - | - | - | - |
| v15-REQ-LANE-SERDE | 11 serde round-trip | v15.P11 | ✓ (P11) | - | - | - | - |
| v15-REQ-KANI-LANE-NAME | 4.2 harness naming | v15.K1 | - | — (K1-kani NOT_VERIFIED: cargo-kani not on PATH; raw log is metadata stub) | - | - | - |
| v15-REQ-ATOMIC-BASELINE-LOAD | 8 atomic load | v15.L1 | - | - | - | — (L1-loom NOT_VERIFIED: loom permutation not actually executed; raw log is metadata stub; newly-written concurrent harness without raw output) | - |
| v15-REQ-INVENTORY-PARSE | 12-H9 inventory parse | v15.F1 | - | - | - | - | — (F1-fuzz NOT_VERIFIED: cargo-fuzz not installed; raw log is metadata stub; source_refs/behavior_test_refs are hallucinated paths) |
| v15-REQ-OUTCOMES-PARSE | 12-H2 outcomes parse | v15.F2 | - | - | - | - | — (F2-fuzz NOT_VERIFIED: cargo-fuzz not installed; raw log is metadata stub; source_refs/behavior_test_refs are hallucinated paths) |

## Coverage

- 17/17 requirements have at least one lane decision.
- Default Rust profile (proptest, kani, verus) covers 16/17 obligations.
- Loom covers 1, cargo-fuzz covers 2.
- No verifier is silently skipped: every seed has explicit applicability.

## Verified / Not-verified tally (audit 2026-07-19, truth-serum)

- Verified PASS (raw command output): 11 proptest obligations (P1, P2, P3-proptest, P4, P5, P6, P7-proptest, P8, P9, P10, P11).
- NOT_VERIFIED: 7 obligations whose raw `exec-*.txt` evidence is a 3-line metadata stub with no captured stdout/stderr, AND whose backing tool (or production symbol) is unavailable / missing on this host:
  - **kani (3 rows)**: P3-kani (LED-004), K2-kani (LED-010), K1-kani (LED-015) — `cargo-kani` binary absent from PATH on this host (`cargo kani list` / `cargo kani --harness` are not invokable); lane writes a `Skipped { reason: ToolUnavailable(ToolKind::CargoKani) }` shape per `.evidence/v1.5/raw/kani-lane-evidence.md`.
  - **verus (1 row)**: V1-verus (LED-007) — the production-binding target `spec_mutant_id_closed_set` is referenced by the ledger but does not exist in `crates/titania-core/src/`; `cargo-verus` (verus 0.2026.05.05) is installed, but there is no spec to verify.
  - **loom (1 row)**: L1-loom (LED-016) — the harness `atomic_baseline_write_under_concurrent_read` is newly written and present in `crates/titania-lanes/tests/v15_atomic_baseline.rs` (#![cfg(loom)] gate), but the loom permutation was never executed; the file's own header documents the canonical gate as compile-only `RUSTFLAGS="--cfg loom" cargo check --tests`. Per evidence-packaging policy, newly-written concurrent harnesses must NOT be marked PASS without raw command output present.
  - **cargo-fuzz (2 rows)**: F1-fuzz (LED-017), F2-fuzz (LED-018) — `cargo-fuzz` not installed on this host; fuzz targets exist as scaffolding stubs at `fuzz/fuzz_targets/fuzz_parse_inventory.rs` and `fuzz/fuzz_targets/fuzz_parse_outcomes.rs`; nightly `cargo fuzz run` was never executed.

Raw log paths in `.evidence/v1.5/raw/exec-*.txt` are PRESERVED as metadata stubs for chain of custody (not deleted).

## Cross-lane balance
- proptest: 11 obligations (pure core) — all verified.
- kani: 4 obligations (bounded symbolic execution) — 0 verified on this host (binary absent); 1 row (P3-proptest binding) covered by proptest.
- verus: 1 obligation (deductive hot-path) — 0 verified (production symbol does not exist).
- loom: 1 obligation (concurrency) — 0 verified (loom permutation not executed).
- cargo-fuzz: 2 obligations (hostile input) — 0 verified (binary absent).

Total planned obligations: 18 lane decisions. Verified: 11. NOT_VERIFIED (raw evidence missing): 7.
