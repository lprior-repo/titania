# Truth-serum report — tn-f7ti

reviewer_skill: truth-serum
reviewer_invocation_id: INV-17-truth-serum
writer_invocation_id: INV-16-evidence-packaging

STATUS: APPROVED

Raw-evidence audit, claim by claim:

- "13 behavior checks green": CONFIRMED in
  evidence/proof_registry-tests.log (exit 0, 13 passed).
- "Strict source clippy exit 0": CONFIRMED in evidence/clippy-src.log.
- "fmt/check/doc exit 0": CONFIRMED in respective logs.
- "Touched crate 262 passed, 1 pre-existing failure": CONFIRMED in
  evidence/titania-lanes-tests.log; the failure reproduces on the base
  commit (stash round-trip witnessed), so the no-regression claim holds.
- "Failing-first": CONFIRMED — the suite predates the module and first
  failed compile (E0432/E0433), then passed after implementation.
- "Executable-check substitution": OPENLY RECORDED in the plan, every
  obligation row, and the waiver candidate; not presented as verifier
  execution. Verus/flux binaries exist but were not run (out of scope).
- "Specialist pipeline": DEGRADED — no named specialist subagents exist
  in this runtime; skill bodies were applied inline and every handoff is
  recorded as fallback without user approval in routing-ledger.jsonl.
  Nothing in the bundle depends on specialist identity for correctness:
  all gates are machine runs with retained logs.

No hallucinated output, no invented paths, no silent waivers. The bundle
is accepted with the two residuals above stated.
