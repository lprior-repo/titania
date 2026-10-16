# Transcript s4.tn-6r36.proof-planner

Skill: proof-planner. Bead: tn-6r36. State: 4. Inputs: contract.md,
proof-seeds.jsonl (9 seeds).

Method: full 5-verifier profile (kani, verus, flux-rs, proptest, cargo-fuzz)
x 9 seeds = 45 lane decisions. Required: 10 proptest + 2 verus + 1 kani +
1 fuzz (F1). not_applicable rows carry concrete evidence refs (contract sha
+ type-design statements). Loom excluded (no concurrency surface), recorded
in proof-strategy.md. 13 planned obligations with exact commands, workdirs,
bounds, assumptions, expected evidence; targets are planned classifier paths
owned by follow-up beads (stated per-obligation). No behavior-affecting
waivers (waiver-candidates.jsonl holds a NONE row). Trusted-base plan
records zero new trusted markers and STRONG-binding rule.

Outputs: verifier-lane-decisions.jsonl, proof-obligations.planned.jsonl,
proof-strategy.md, verifier-lane-matrix.md, proof-coverage-matrix.md,
trusted-base-plan.md, waiver-candidates.jsonl, delivery-scope.jsonl.

No reviewer disposition stamped here; review belongs to s4b.
