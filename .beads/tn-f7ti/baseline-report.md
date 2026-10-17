# Baseline report — tn-f7ti

Worktree base: 927893f922fe2b9c658a0c1574086afe7d6a6dc6 (origin/main).
Bead claimed from OPEN state; no blockers on the bead itself.

## Pre-existing tree state

- contracts/proof_obligations.yaml listed two proof targets with no
  authority distinction: the setup smoke fixture and the receipt proof were
  indistinguishable to any consumer.
- No registry-completeness control existed: nothing failed a missing,
  proof-free, smoke-only, or removed obligation, and nothing flagged
  unregistered production proof.
- One touched-crate test (dylint_reject_repair) fails on the untouched base
  (verified via stash round-trip); it is unrelated to this bead.

## Scope boundary

This bead delivers the audit control plus its behavior evidence. Full
`moon ci` and kani/mutant lanes are pre-existing scope owned by tn-4isu and
tn-7bq2 and are explicitly out of scope for this delivery.
