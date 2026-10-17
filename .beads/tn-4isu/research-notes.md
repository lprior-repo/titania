# Research notes — tn-4isu

## Files read (exact refs)

- `crates/titania-core/src/kani.rs` (535 lines): 12 harnesses listed in STATE.md.
  Heavy bounds: `#[kani::unwind(100)]` on `kani_kani_harness_id_bounded` (line 144),
  `#[kani::unwind(32)]` on `kani_kani_lane_name_roundtrip` (line 279) and
  `kani_mutants_baseline_diff_zero_neg` (line 370), `#[kani::unwind(64)]` on
  `outcome_wire_invariants` (line 519).
- `crates/titania-core/src/proof_id.rs`: `KANI_HARNESS_ID_MAX_LEN = 96` (line 18);
  `check_khi` iterates `bytes[1..]` once via `try_for_each` (line 114) — max loop
  trip for any accepted input is 96, for harness-9 inputs (≤32 bytes) is 32.
  No loops in `KaniHarnessId::new` beyond that single byte scan + `s.to_owned()`
  (memcpy, bounded by 96).
- `crates/titania-core/src/mutants_baseline.rs`: `contains` = `iter().any(...)`
  over ≤8 entries (line 117-119); `diff` = `iter().filter(...)` over ≤8 survivors
  (line 126-128). Max loop trip ≈ 8×8 = 64 inner evaluations, loop nesting depth 2
  but each loop ≤8 iterations. `unwind(32)` is ~4× over-approximation.
- `crates/titania-lanes/src/run_lane_kani.rs`: per-package model confirmed —
  ONE `cargo kani -p <pkg>` per package (doc lines 13-18), wallclock cap
  `PER_PACKAGE_TIMEOUT_SECS = 600` (line 89), cgroup `MemoryMax=24G`,
  `MemorySwapMax=0` (lines 103-104, doc 15-16), pipe cap 8 MiB (line 94).
  Timeout ⇒ every harness in package recorded `PROOF_KANI_BLOCKED`.
- `.moon/tasks/all.yml`: `titania-kani` task runs
  `cargo run -p titania-check -- run-lane kani` with `CARGO_TARGET_DIR=.titania/cache/kani`.

## Patterns searched

- `grep -rn "kani::proof" crates/` → 12 hits, all in `titania-core/src/kani.rs`.
  No other crate carries harnesses. Required inventory = 12, single package.
- `grep -rn "KANI_HARNESS_ID_MAX_LEN"` → contract cap 96; harness-9 contract bound
  `KANI_ID_CONTRACT_BOUND = 32` with global-overflow case at exactly 97 bytes.
- `cargo kani --version` → `cargo-kani 0.67.0`. `rustc 1.97.0-nightly (ca9a134e0 2026-04-26)`.
  `systemd-run --user` available.

## Diagnosis (model-size analysis, no rerun of the timed-out full run)

- The 12/12 timeout is a per-package artifact: one `cargo kani -p titania-core`
  invocation holds all 12 harnesses; CBMC processes harnesses sequentially with
  `-j 1`, so the three symbolic harnesses (9, 11, 12) consume the 600s budget
  and the 8 trivial + 1 moderate harness never report.
- Harness 9 (`unwind(100)`): loops present are `from_utf8` over ≤32 bytes,
  `symbolic_bytes.map` (32, unrolled const), `check_khi` scan over ≤32 bytes,
  fixed `CONCAT` asserts (no loops). Required unwind ≈ 33. Bound 100 triples
  CBMC unwinding assertions and symex state for zero additional coverage.
  SOUND reduction: 100 → 40 (margin over 32 + 97-byte overflow path, which is a
  fixed string, not a loop — its scan is ≤97 iterations: max loop trip overall
  is the 97-byte `check_khi` scan ⇒ bound must be ≥ 98 to cover the overflow
  case's byte scan!). Correction: OVER_GLOBAL_LIMIT is 97 bytes; `check_khi`
  scans bytes[1..] = 96 iterations via try_for_each. So unwind must be ≥ 97.
  Reduction 100 → 97 is marginal. Better lever for harness 9: none from unwind;
  keep 100. Its cost driver is 34 symbolic bytes, not unwind.
- Harness 11 (`unwind(32)`): production loops ≤8 iterations; harness-level
  `(0..8).for_each` closures are also ≤8. Required unwind ≈ 9-16 (filter+any
  nesting). Reduction 32 → 16 is sound-by-construction (Kani unwinding
  assertions fail loudly if insufficient). Cost driver is 24 symbolic bools +
  16 heap Strings + Vec; unwind halving cuts symex branching of loop
  unrollings.
- Harness 12 (`unwind(64)`): serde_json parse over ≤8 skad bytes; serde internal
  loops bounded by input len (8) plus small constant state machines. Reduction
  64 → 24 candidate; must verify empirically (Kani will fail loudly if short).
- Harness 10 (`unwind(32)`): iterates Full lanes (small closed set), string
  building over short stems; likely already fast. Leave unless triage shows cost.

## Triage plan (resource-compliant)

Per-harness runs, `-j 1`, inside the mandated cgroup, short per-harness
timeouts (120-300s), lightest-first:

1. Harness 3 (representative u32-assume shape) — expect seconds.
2. Harness 10 — expect fast; confirms moderate harnesses are not the blocker.
3. Harness 12 with current unwind(64), cap 300s — measure.
4. Harness 11 with current unwind(32), cap 300s — measure.
5. Harness 9 with current unwind(100), cap 300s — measure.
6. Apply sound unwind reductions (11→16, 12→24), re-run; keep 9 at 100 unless
   data shows unwind (not symbolic bytes) dominates.

If a harness still cannot complete in bounds: record exact evidence, keep bead
open/blocked with numbers. No faked passes.

## Consumer-visible regression proving the control

`titania-check run-lane kani` per-package run completing within 600s with 12/12
`VERIFICATION: SUCCESSFUL` verdicts on the current revision — the exact
artifact the Full gate consumes (`.titania/out/full/kani.json`).

## Duplicate/overlap check

- `tn-7bq2` (parent milestone): depends on this bead; not a duplicate.
- `tn-mudh` (closed): ground-truth reporter; not a duplicate.
- No other open bead owns Kani solver bounds.
