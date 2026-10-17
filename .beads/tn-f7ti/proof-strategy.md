# Proof strategy — tn-f7ti

This bead is coverage integrity, not executable-correctness proof. The
defense-in-depth lanes therefore bind to executable behavior evidence over
the audit control itself.

Lane posture per seed (default Rust behavior profile: verus, kani,
flux-rs, proptest — all required, none silently omitted):

- verus lane: required. The control is ordinary safe Rust with no verus
  specs of its own; the lane obligation is satisfied by executable checks
  over the same claim (mode executable-check), with the substitution
  recorded in each obligation row. verus 0.2026.05.05 is present but
  executing it is out of the delivery scope.
- kani lane: required. Same executable-check posture; referenced artifacts
  carry real assertions and no cover-only content.
- flux-rs lane: required. Same executable-check posture.
- proptest lane: required. Same executable-check posture over the 13-check
  behavior suite, which exercises happy, error, and edge clauses with exact
  assertions.

No lane is blocked_tooling and none is waived: every required obligation
closes with a PASS ledger row pointing at a real executed command, real
exit status, and a real log artifact. The substitution rationale is part
of the reviewed plan, not a silent downgrade.
