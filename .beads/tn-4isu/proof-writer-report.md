# Proof-writer report — tn-4isu (State 5)

## Artifacts written (proof/harness only, zero production change)

`crates/titania-core/src/kani.rs` — 3 lines changed, all inside `cfg(kani)` harness scope:

1. Line 370: `#[kani::unwind(32)]` → `#[kani::unwind(16)]` on
   `kani_mutants_baseline_diff_zero_neg` (obligation OBL-DIFF, seed P1).
2. Line 519: `#[kani::unwind(64)]` → `#[kani::unwind(24)]` on
   `outcome_wire_invariants` (obligation OBL-WIRE, seed P2).
3. Line 517: doc comment `unwind(64)` → `unwind(24)` (consistency; prose only).

`git diff` confirms: 1 file, 3 insertions, 3 deletions. No assert/assume/cover,
input-shape, production, lane, or config line touched (INV-1 holds by inspection).

## Bound justification (recorded before execution; execution decides)

- Diff harness: production loops trip ≤ 8; compared strings ≤ 15 bytes
  (`entry_covers` compares `mutation_id` only, max `p::a:8:1:and_or` = 15).
  Bound 16 keeps margin; Kani unwinding assertions stay enabled so an
  insufficient bound FAILS loudly (see triage note: a 30-iteration memcmp site
  exists in the pre-change log — the post-change run must show no unwinding
  failure or this bound reverts to 24).
- Wire harness: input ≤ 8 bytes; pre-change run stalled unwinding
  `run_utf8_validation` past iteration 32. Bound 24 cuts symex ~2.7×; revert
  to 32 on any unwinding failure.
- Id harness: unchanged at 100 (97-byte fixed overflow scan needs ≥ 97).

## Smoke evidence (cheap, pre-formal)

- `git diff --stat`: 1 file, 3+/3- (above).
- Typecheck of the harness file runs with the normal gate
  (`cargo check --workspace --all-targets --all-features`, State 12) since
  `kani.rs` compiles under `cfg(kani)` via the kani toolchain, not rustc:
  syntax risk is closed by the per-harness Kani runs themselves (a syntax error
  fails the run before verification). No PENDING_FORMAL_EXECUTION remains:
  formal execution is State 12 with exact commands from
  `proof-obligations.planned.jsonl`.

No tooling blockers. No new trust markers beyond T1/T2 model bounds.
