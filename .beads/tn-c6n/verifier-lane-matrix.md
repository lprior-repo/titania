# Verifier lane matrix — tn-c6n

| Seed | Requirement | verus | kani | flux-rs | proptest |
|---|---|---|---|---|---|
| tn-c6n.S1 | R1 lane-name validity | n/a (String/serde outside fragment) | n/a (owned by tn-7bq2; tool absent) | n/a (no annotations; serde outside fragment) | required → O-P1 |
| tn-c6n.S2 | R2 lane-digest consistency | n/a (same) | n/a (same) | n/a (same) | required → O-P2 |
| tn-c6n.S3 | R3 error exhaustiveness | n/a (same) | n/a (same) | n/a (same) | required → O-P3 |
| tn-c6n.S4 | R4 period ordering | n/a (same) | n/a (same) | n/a (same) | required → O-P4 |
| tn-c6n.S5 | R5 target-root validity | n/a (same) | n/a (same) | n/a (same) | required → O-P5 |
| tn-c6n.S6 | R6 schema gate | required → O-V6 (binding hygiene) | n/a (same) | n/a (same) | required → O-P6 |

n/a rows carry concrete evidence refs in `verifier-lane-decisions.jsonl` and
require reviewer acceptance before proof writing. No `blocked_tooling` rows:
every lane is either executed with raw output or ruled out with evidence.
