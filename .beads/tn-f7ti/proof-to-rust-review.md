# Proof-to-Rust bridge review — tn-f7ti

reviewer_skill: proof-reviewer
reviewer_invocation_id: INV-09-proof-reviewer-bridge
writer_invocation_id: INV-08-proof-to-implementation

STATUS: APPROVED

Adversarial pass over the bridge: every behavior-affecting proof
obligation has exactly one refinement row preserving requirement, clause,
and verifier identity; source refs are concrete path::symbol refs into
production code (no test or harness paths); behavior test refs are
independent executable checks, disjoint from the harness refs; no verifier
harness stands in as a behavior test; mapping status is verified with
executed command evidence on every row. No copy/reality gap: the suite
calls production entry points directly.
