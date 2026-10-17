# Proof Strategy — tn-bvv7

Two proof seeds, both behavior-affecting and implementation-bound
(detector predicates in production source, no copied models):

- SEED-S1 (REQ-3): fully-qualified / renamed-import / macro-hidden
  legacy effects still fire. Executable lane: proptest
  (`tn_bvv7_fq_bypass_property`, 64 cases, exit 0) plus exact detector
  tests, catalog content tests, and lane rejection tests.
- SEED-S2 (REQ-1): new process/async/state boundaries fire under noise
  while the frozen-catalog allowance stays silent. Executable lane:
  proptest (`tn_bvv7_effect_boundary_property`, 64 cases, exit 0) plus
  exact detector/scope tests and lane tests.

Non-applicable lanes (each with concrete evidence, reviewer-accepted):
verus (no invariants beyond exact boundary predicates), kani
(parent-scoped to tn-4isu/tn-7bq2; predicates are loop-free total
functions), flux-rs (no index/length refinements; checked splits only),
loom (detectors are synchronous pure functions; nothing executes
concurrently), miri (no unsafe in new or touched code; workspace
forbids it).

No verification harness doubles as a behavior test: RROs cite
independent integration tests and separate property harnesses with
disjoint refs. No waivers for behavior-affecting rows.
