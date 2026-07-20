# Proof Evidence (State 5)

## Source-of-truth linkages

Every obligation has its source anchor recorded in
`proof-to-implementation-input.md` (State 4 artifact). Proof-writer
emitted test/harness artifacts at those paths; this file is the
artifact index.

## Per-obligation evidence index

| Obligation | Evidence file path | Command | Expected exit |
|------------|--------------------|---------|----------------|
| v15-OBL-P1-LANE-ROUNDTRIP | crates/titania-core/tests/v15_lane_roundtrip.rs | cargo test -p titania-core --test v15_lane_roundtrip | 0 |
| v15-OBL-P2-GATESCOPE-ROUNDTRIP | crates/titania-core/tests/v15_gate_scope_roundtrip.rs | cargo test -p titania-core --test v15_gate_scope_roundtrip | 0 |
| v15-OBL-P3-KANI-ID-PROPTEST | crates/titania-core/tests/v15_kani_harness_id.rs | cargo test -p titania-core --test v15_kani_harness_id | 0 |
| v15-OBL-P3-KANI-ID-KANI | crates/titania-core/src/kani.rs | cargo kani -p titania-core --harness kani::kani_kani_harness_id_bounded | 0 |
| v15-OBL-P4-KANI-ID-SERDE | crates/titania-core/tests/v15_kani_harness_id_serde.rs | cargo test -p titania-core --test v15_kani_harness_id_serde | 0 |
| v15-OBL-P5-MUTANT-ID-PROPTEST | crates/titania-core/tests/v15_mutant_id.rs | cargo test -p titania-core --test v15_mutant_id | 0 |
| v15-OBL-V1-MUTANT-ID-VERUS | crates/titania-core/src/proof_id.rs::spec::spec_mutant_id_closed_set | cargo verus --verify-fn spec_mutant_id_closed_set | 0 |
| v15-OBL-P6-MUTANTS-LOAD | crates/titania-core/tests/v15_mutants_baseline_load.rs | cargo test -p titania-core --test v15_mutants_baseline_load | 0 |
| v15-OBL-P7-MUTANTS-DIFF-PROPTEST | crates/titania-core/tests/v15_mutants_baseline_diff.rs | cargo test -p titania-core --test v15_mutants_baseline_diff | 0 |
| v15-OBL-K2-MUTANTS-DIFF-KANI | crates/titania-core/src/kani.rs | cargo kani -p titania-core --harness kani::kani_mutants_baseline_diff_zero_neg | 0 |
| v15-OBL-P8-MUTANTS-EXPIRY | crates/titania-core/tests/v15_baseline_expiry.rs | cargo test -p titania-core --test v15_baseline_expiry | 0 |
| v15-OBL-P9-SKIP-REASON-TOOL | crates/titania-core/tests/v15_skip_reason_tool_unavailable.rs | cargo test -p titania-core --test v15_skip_reason_tool_unavailable | 0 |
| v15-OBL-P10-LANE-NAME | crates/titania-core/tests/v15_lane_name.rs | cargo test -p titania-core --test v15_lane_name | 0 |
| v15-OBL-P11-LANE-SERDE | crates/titania-core/tests/v15_lane_serde_roundtrip.rs | cargo test -p titania-core --test v15_lane_serde_roundtrip | 0 |
| v15-OBL-K1-KANI-NAME-KANI | crates/titania-core/src/kani.rs | cargo kani -p titania-core --harness kani::kani_kani_lane_name_roundtrip | 0 |
| v15-OBL-L1-ATOMIC-LOAD-LOOM | crates/titania-lanes/tests/v15_atomic_baseline.rs | RUSTFLAGS="--cfg loom" cargo test --release -p titania-lanes --test v15_atomic_baseline | 0 |
| v15-OBL-F1-FUZZ | fuzz/fuzz_targets/fuzz_parse_inventory.rs | cargo +nightly fuzz run fuzz_parse_inventory -- -max_total_time=300 | 0 |
| v15-OBL-F2-FUZZ | fuzz/fuzz_targets/fuzz_parse_outcomes.rs | cargo +nightly fuzz run fuzz_parse_outcomes -- -max_total_time=300 | 0 |

## Lane profile summary

- proptest: 11 obligations.
- kani: 4 obligations (3 new + 1 from v15.P5).
- verus: 1 obligation.
- loom: 1 obligation.
- cargo-fuzz: 2 obligations.

## Cgroup / cfg(loom) indirection

- Kani: every `cargo kani` invocation wrapped in `systemd-run --user
  --scope --collect -p MemoryHigh=20G -p MemoryMax=24G
  -p MemorySwapMax=0`. If systemd-run is unavailable on host, lane
  emits typed warning finding `PROOF_KANI_PASS` with
  `host-cgroup: absent` metadata; gate stays green.
- loom: target code path switches to `loom::sync::*` under
  `cfg(loom)`; production code remains `std::sync::*`.

## No behavior-affecting waiver

- proof-test-source-alignment will show that every proptest test name
  has a behavior-test counterpart.
- Kani harnesses target production functions (no copied models).
- Verus spec binds to production via `#[path]` and
  `assume_specification`.

## Resolutions to anti-vacuity

- `kani::cover!` is paired with `kani::assert!` for safety properties.
- `cargo mutants` runs in test mode (not `--check`).
- Loom runs multiple preemption modes; only `LOOM_MAX_PREEMPTIONS<=2`
  per skill rule.
