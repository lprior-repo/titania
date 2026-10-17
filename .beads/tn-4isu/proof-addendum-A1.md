# Proof-writer addendum A1 — wire 24→12 (State 5 supplement)

Supersedes the T2 value recorded in proof-writer-report.md (which described the
state of the tree at State-5 close: wire @24). No pinned file is edited by this
addendum; the final tree state is `#[kani::unwind(12)]` + synced comment.

## Change

`crates/titania-core/src/kani.rs:517-519`: `unwind(24)` → `unwind(12)` (+ comment).
Same inputs, same shaping, same asserts, same cover points. Attribute-only, as D4/D5.

## Trigger

Pre-registered revert/escalate condition fired with numbers: wire @24 TIMEOUT at
400s cap (evidence/kani-verify-wire-u24.log, EXIT 124, 1105 unwind steps, solver
phase never completed). Bound 24 is not sufficient-COST (distinct from
sufficient-CORRECTNESS, which was never in doubt).

## Sufficiency argument for 12 (to be closed by execution)

Input ≤ 8 bytes; `run_utf8_validation` needs ≤ 9 iterations on this domain;
serde_json parse/whitespace/number loops are input-driven (≤ 8 digits);
`\u`-escape handling is fixed-count (4). Unwinding assertions stay enabled: if
12 is insufficient the run FAILS loudly naming the loop, and T2 reverts to 24.

## Review status

Covered by the State-13 black-hat review of the final tree; the State-6 review
remains valid for everything except the T2 numeral (disclosed here). Trust
ledger: T2 updated in place (unpinned at the time), T7 memorializes the @24
attempt with its timeout log.
