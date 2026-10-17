# Assurance Bundle — tn-bvv7 (evidence-packaging)

Requirement → contract → proof/refinement/test/source → raw evidence:

- REQ-1 (reject forbidden capabilities): contract clauses CC-1/CC-4;
  O2+R2 (proptest PASS, ledger closed); source `detectors.rs`
  process/async/state; raw `evidence/proptest-obligations.txt`,
  `evidence/touched-crates-test.txt`.
- REQ-2 (permit deterministic core + shell boundary): contract CC-2;
  scope tests + allowed fixtures + lane clean controls (all in the
  touched-crate log); source `filters.rs` (unchanged logic).
- REQ-3 (renamed/direct clock/fs/env/network blocks): O1+R1
  (proptest PASS, ledger closed); FQ/alias/macro fixtures; pre-fix
  failure log `evidence/failing-first-detectors.txt` proves the hole
  was real and is now closed.
- Unwanted (no completion on absent/contradicting evidence): every
  claim above cites an exit code and a retained log; moon ci and
  kani/mutants are recorded as not run, never as passed.

Gates: fmt 0, check 0, strict clippy 0, tests 848/0, doc 0
(logs in `evidence/`). Reviews: plan, proof, bridge, tests,
black-hat — all STATUS: APPROVED with invocation provenance.
Landing: NOT performed (no commit/push/sync authorized); worktree
handoff only.
