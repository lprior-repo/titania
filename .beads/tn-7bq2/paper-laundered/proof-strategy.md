# Proof Strategy (State 4 — proof-planner)

## Lane profile (default Rust)

- `verus`, `kani`, `flux-rs`, `proptest` are the default obligators.
  Each seed must specify applicability per default verifier.
- `loom` is conditional on concurrency/atomic/race risk in seed text.
- `miri` is conditional on unsafe/UB/FFI risk in seed text.
- `cargo-fuzz` is conditional on parser/hostile-input/wire risk in seed
  text.

## v1.5 obligation set

17 seeds; 18 active verifier-lane-decisions (because 1 seed has 2 lanes); 35
`not_applicable` markers capture the other required verifier tuples per
seed, ensuring no silent omission.

## Total obligations

- 18 `required` lanes (proptest-heavy on validators: empirical coverage
  on every seed; kani on 4; verus on 1; loom on 1; cargo-fuzz on 2).
- 35 `not_applicable` lanes (evidence-refs to
  .evidence/v1.5/raw/v15-*-NA.md).

## Anti-laundering

- No `assume`, `axiom`, `admit`, `external_body` in any executable proof
  code.
- No `cover!`-as-proof for safety, equality, ordering obligations.
- No copied harness models without bridge rows.
- No generic waivers; behavior-affecting obligations all have a real
  verifier.

## Resource governance

- Kani runs are `-j 1` cgroup-capped at `MemoryMax=24G`,
  `MemorySwapMax=0` per the Kani skill rule.
- fuzz runs are `1` job with `-max_total_time=300` (5-minute budget per
  session).
- loom runs respect `LOOM_MAX_PREEMPTIONS=2` for bounded exploration.

## Waiver posture

No behavior-affecting waivers. The single waiver-candidate row marks
"NONE — no waivers planned".

## Mapping status

All obligations `mapping_status: planned` (State 4). The mapping is
materialized in State 7 (proof-to-implementation) and verified in
State 12 (formal-verifier).
