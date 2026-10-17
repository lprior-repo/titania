# Truth-Serum Report — tn-bvv7

reviewer_skill: truth-serum
reviewer_invocation_id: s14b.tn-bvv7.truth-serum
writer_invocation_id: s14a.tn-bvv7.evidence-packaging

STATUS: APPROVED

Dual-persona audit (claims vs raw evidence):
- "4 FQ bypasses existed" — VERIFIED (`evidence/failing-first-detectors.txt`
  shows 4 named failures on the pre-fix code).
- "848 tests pass" — VERIFIED (`evidence/touched-crates-test.txt`,
  summed `test result: ok` lines, zero failures).
- "gates exit 0" — VERIFIED (each `evidence/gate-*.txt` ends `EXIT:0`).
- "proptest 2×64 pass" — VERIFIED (`evidence/proptest-obligations.txt`).
- "no unsafe added" — VERIFIED (workspace forbid untouched; diff adds
  no unsafe blocks; checked via `git diff` inspection).
- "dylint intact" — VERIFIED (`policy_surface` 10/10 in the same log).
- "moon ci / kani / mutants pass" — NOT CLAIMED (recorded as not run).
- "/tmp quota caused receipt/dylint flakes" — VERIFIED (identical
  failure on untouched base 927893f; passes with shm TMPDIR).
- "single-operator execution" — DISCLOSED as debt tn-bvv7.F-IND-1;
  validator provenance and ledger record inline execution, no
  specialist impersonation.

No hallucinated commands, outputs, or paths. Every number above was
read back from its cited file in this worktree.
