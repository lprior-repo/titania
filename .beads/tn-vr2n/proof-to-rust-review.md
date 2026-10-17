# Proof-to-Rust review — tn-vr2n

reviewer_skill: proof-reviewer
reviewer_invocation_id: s7b.tn-vr2n.proof-reviewer-bridge

STATUS: APPROVED

All three behavior-affecting obligations carry a bridge row preserving
requirement/contract/verifier identity. Every source ref is a concrete
`path::symbol` that exists; every behavior test ref is an independent
executable suite (no verifier-harness paths, no forbidden vocabulary);
refinement refs are disjoint files exercising the same production code at
a second level. No copied models, no shadow types: tests call production
APIs directly, so the bridge cannot diverge.

Residual: review performed inline by the single delivery operator
(parent-delegated fallback). Genuinely independent review pending.
