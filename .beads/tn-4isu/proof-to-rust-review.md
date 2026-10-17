# Proof-to-Rust bridge review — tn-4isu

reviewer_skill: proof-reviewer
reviewer_invocation_id: inv-s07b-bridgereview
writer_invocation_id: inv-s07-bridge
review_state: 7
reviewed_artifacts: proof-to-rust-map.md, rust-refinement-obligations.jsonl

## Checks

1. Every obligation (OBL-DIFF/WIRE/ID/FULL, all behavior_affecting=false) has a
   bridge row preserving requirement/contract identity and verifier. PASS.
2. Source refs are concrete `path::symbol` refs into the harness file. PASS.
3. `behavior_test_refs: []` on all rows is CORRECT here, not a gap: the change
   is unwind attributes on `cfg(kani)` harnesses — no executable behavior
   exists that a behavior test could observe. Claiming otherwise would be proof
   theater. The harness verdicts are the evidence (State 12). PASS.
4. No bridge row relabels a behavior change as proof-only: `git diff` proves
   production code untouched. PASS.

## STATUS: APPROVED
