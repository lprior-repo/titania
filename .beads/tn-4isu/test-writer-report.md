# Test-writer report — tn-4isu (State 9)

## Proof/Refinement Coverage Matrix

| Proof/Refinement ID | New test written | Justification |
|---|---|---|
| tn4isu.OBL-DIFF | none | proof-only change; existing 35 path-range tests guard the domain |
| tn4isu.OBL-WIRE | none | proof-only change; existing 22 outcome_serde tests guard the domain |
| tn4isu.OBL-ID | none | proof-only change; existing 8 unit tests guard the domain |
| tn4isu.OBL-FULL | none | proof-only change; full 96-test suite green |

## Execution evidence

`cargo test -p titania-core --all-features` → exit 0; 6 binaries, 96 passed,
0 failed (evidence/cargo-test-core.log). No test added, none weakened, none
rewritten. Test implementation style untouched; exact assertions in the existing
suite retained as-is.
