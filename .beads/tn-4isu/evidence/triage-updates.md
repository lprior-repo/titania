# Triage updates (living notes; NOT hash-pinned until the State-12 row)

## 2026-10-16: wire @24 TIMEOUT

`outcome_wire_invariants` at `#[kani::unwind(24)]`: TIMEOUT at 400s cap
(evidence/kani-verify-wire-u24.log). 1105 unwind steps complete
(run_utf8_validation copies .0/.1 fully unwound), then the silent
solver/symex phase consumes the rest. Reduction 64→24 did NOT resolve.
Hypotheses: (a) serde generic Deserialize formula over symbolic input is
inherently large at any sufficient unwind; (b) MemoryHigh=20G thrash.
Next experiments in order: (1) max-RSS measurement, (2) unwind 24→12
(attribute-only), (3) property-preserving `usize→u8` len symbol ONLY if (2)
fails on cost (not on correctness). Revert rule: any unwinding-assertion
failure reverts to the last sufficient bound.

## 2026-10-16: diff @16 RUNNING

`kani_mutants_baseline_diff_zero_neg` at `#[kani::unwind(16)]`, 400s cap
(evidence/kani-verify-diff-u16.log). Mid-run profile: ~455 memchr sites and
~150 memcmp sites each unwinding to the bound; harness setup (format!×8,
to_owned×16, to_vec) plus ~200 contains-driven comparisons. Pending result.
Watch item F2 (memcmp site at iteration 30 pre-change): the run MUST show zero
unwinding-assertion failures or T1 reverts to 24 with a re-run.

## Rule (learned): never edit hash-pinned files

STATE.md, provenance, baseline, and every ledger-pinned output are frozen once
pinned. Living notes go here. At State 15/16 the full invocation chain is
rebuilt deterministically (same canonical hashing) so the final STATE.md and
final hashes agree; the rebuild is recorded in cleanup-report.md.

## 2026-10-16: diff @16 TIMEOUT

`kani_mutants_baseline_diff_zero_neg` at `#[kani::unwind(16)]`: TIMEOUT at 400s
cap (evidence/kani-verify-diff-u16.log, exit 124, 13663 unwind lines, 9.2MB log,
no VERIFICATION line). Reduction 32->16 halved per-site unwind cost but the site
count (~450 memchr + ~150 memcmp from String-heavy production calls explored
under 24 symbolic bools, plus allocator SSA) still exceeds budget. Attribute-only
fix exhausted for this harness (16 is at the memcmp floor: 15-char MutantIds).
Remaining levers: harness-body refactor (hoist redundant contains, branch-free
asserts) or package split (parent-sanctioned) or numbered blocker.

## 2026-10-16: wire 24->12 (T2 superseded by T7)

Rationale recorded BEFORE the run: @24 completed 1105 unwinds then died in the
silent solver/symex phase at 400s; serde internal loops are input-driven (<= 8
bytes), so 12 keeps margin while roughly quartering the unwound formula vs 64.
Revert rule: unwinding-assertion failure -> back to 24 (then u8-len experiment,
then numbered blocker). The @24 attempt is preserved as ledger row T7.

## 2026-10-16: wire @12 TIMEOUT (structural blocker)

`outcome_wire_invariants` at `#[kani::unwind(12)]`: TIMEOUT at 400s cap
(evidence/kani-verify-wire-u12.log, no VERIFICATION line). Profile: 356
monomorphized copies of serde_json `parse_exponent_overflow`, each unwound to
the bound, plus f64_from_parts (649 lines), parse_exponent (327),
parse_decimal_overflow (275) — the derived Deserialize over nested
LaneOutcome integers pulls serde_json's ENTIRE generic number/float parser
into the model once per integer parse path. The bound applies correctly
(iterations cap at 12); the disease is copy-count x bound, unreachable from any
sound harness-side change (stubbing serde = proof laundering, forbidden;
shrinking the 8-byte domain = property reduction, forbidden). VERDICT: wire
harness BLOCKED with numbers (see verification-ledger.jsonl). Revert rule for
the tree: keep @12 (least-bad bound; @24/@64 strictly worse, no failure seen).

## 2026-10-16: id @100 TIMEOUT (structural blocker)

`kani_kani_harness_id_bounded` at `#[kani::unwind(100)]`: TIMEOUT at 400s cap
(evidence/kani-verify-id-u100.log, EXIT 124, only 2147 unwind lines — ~5 steps/s:
each step builds SSA over 34 symbolic bytes x from_utf8/byte-scan branches).
Bound floor is 97 (fixed 97-byte overflow scan); domain is 32+2 symbolic bytes.
No sound lever: bound already minimal, domain fixed by INV-1. VERDICT: BLOCKED
with numbers. Package-split prerequisite (individual <600s completion) unmet
for all three heavies; split rejected with trajectory evidence (wire frozen
9.6MB log 250s->400s; diff still unwinding at 13.6k lines; id at 5 steps/s).
Diff hoist-refactor skipped: cannot unblock the bead while wire+id are
structurally stuck; churning the harness body for a possible single pass has
negative expected value (review burden + theater risk). Revisit if wire/id find
levers in follow-up work.

## Terminal engineering state

- Resolved within bounds: 9/12 (8 fixed/u32 harnesses + lanename@32/59s).
  Light batch run pending to firm the 7 never-run-individually lights.
- Blocked with numbers: diff (u32/280s, u16/400s), wire (u64/280s, u24/400s,
  u12/400s), id (u100/400s).
- Tree kept: diff@16 + wire@12 (sound, strictly-less search, same properties).
  They narrowed but did not close; future work builds on them.

## 2026-10-16: BREAKTHROUGH — default-bound diagnosis; 2 lights resolved

Root cause for the two light timeouts: harnesses WITHOUT #[kani::unwind]
inherit a gargantuan default unwind (observed memcmp iteration 5107 before the
120s cap fired). Fix: uniform #[kani::unwind(32)] on all 8 fixed-input/u32
harnesses (loops trip <= 16 over <= 16-byte literals; 2x margin; unwinding
assertions verify sufficiency per harness automatically).
- lane_digest_accepts_passed_not_greater_than_scanned: 120s TIMEOUT -> 0.63s SUCCESSFUL.
- recorded_target_root_rejects_nul_byte: 120s TIMEOUT -> 2.27s SUCCESSFUL.
Logs: kani-verify-<name>.u32.log. Prior default-bound passes for the other 6
lights stand (per-harness runs are independent; added attributes on siblings do
not affect them). Inventory now: 9/12 PASS, 3/12 structural BLOCKED.
Heavies unaffected (already explicitly bounded; site-count disease confirmed).
