# tn-4isu Contract — Bounded Kani Solver Repair

Bead: `tn-4isu` (P0). Canonical contract for proof-planner/test/implementation
lanes. No production-behavior change: repair surface is `#[kani::unwind]`
bounds on `cfg(kani)` harnesses in `crates/titania-core/src/kani.rs`.

## Decisions

| # | Decision | Choice |
|---|----------|--------|
| D1 | Repair lever | Unwind-bound reduction only; same harnesses, same inputs, same asserts. |
| D2 | Soundness guard | Kani unwinding assertions stay enabled; a short bound fails loudly, never passes silently. |
| D3 | Harness 9 (`unwind(100)`) | Diagnose-only; bound already near-minimal for the fixed 97-byte overflow scan. |
| D4 | Harness 11 (`unwind(32→16)`) | Reduce: production loops ≤ 8 iterations; 16 keeps 2× margin. |
| D5 | Harness 12 (`unwind(64→24)`) | Reduce pending empirical sufficiency run; revert if Kani reports unwinding failure. |
| D6 | Forbidden | Timeout increase, cap lift, harness drop, property weakening, behavior substitution, `cover!`-as-proof. |
| D7 | Execution | Per-package `-p titania-core -j 1` inside MemoryHigh=20G/MemoryMax=24G/MemorySwapMax=0; never workspace-wide. |
| D8 | Done criterion | 12/12 `VERIFICATION: SUCCESSFUL` within the lane's 600s per-package budget on current revision, or an explicit numbered blocker. |

## Invariants

- INV-1: No required property is reduced; every assert/assume/cover stays byte-identical except the unwind attribute.
- INV-2: Timeout/OOM/SKIP/UNPROVEN never counts as evidence; Full stays blocked until real passes exist.
- INV-3: Moon is the only CI/CD orchestrator.
- INV-4: Listings and `cover!` are non-vacuity evidence, never execution proof.
