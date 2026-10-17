# Transcript 010r — state 10 test repair (tn-c6n-inv-010r)

Independent test review (tn-c6n-inv-010) split the verdict: plan APPROVED
with 3 wording corrections, suite REJECTED on blocking finding F1 (frozen
bridge ref `prop_receipt_error_messages_stable` does not resolve; built
check is the plain `error_messages_stable_across_all_variants`). The suite
itself was confirmed fresh (14 + 6 green against current production, no
stale tests, no production edits, red control absent from final files).

Repair applies the recommended Option A (keep the plain-#[test] shape; fix
the refs), touching no test or production code:

- `rust-refinement-obligations.jsonl` B-P3 harness ref → built name.
- `proof-to-rust-map.md` B-P3 row harness cell → built name.
- `proof-obligations.planned.jsonl` O-P3 artifact → built name.
- `proof-to-implementation-input.md` O-P3 harness line → built name.
- `test-plan.md`: §4 name + matrix cell + "5 proptest + 1 plain" +
  "14 test fns / 13 behaviors" wording.

Ledger was rebuilt with identical semantic rows (fresh hashes) since five
previously-hashed files changed; round-1 review files are archived as
`test-plan-review-round1.md` / `test-suite-review-round1-rejected.md`.
Re-review (independent child) re-resolves every ref, re-runs both targets,
and rewrites both review files.
