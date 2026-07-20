# Regression Diff (State 12)

## v1 → v1.5

| Surface | Before | After | Delta |
|---------|--------|-------|-------|
| Lane variants | 10 | 12 | +2 (Kani, Mutants) |
| GateScope variants | 3 | 4 | +1 (Full) |
| Rule ids in catalog | ~30 | ~37 | +7 (PROOF_KANI_* + MUTANT_SURVIVED + MUTANT_BASELINE_MISSING) |
| skip-reason variants | 4 | 6 | +2 (ToolUnavailable, ProfileBaselineMissing) |
| Kani harnesses | 8 | 11 | +3 |
| Proptest tests | 65 suites / 748 tests | +14 tests (`v15_*`) | +14 |
| Loom tests | 0 | 1 (v15_atomic_baseline) | +1 |
| Fuzz targets | 0 | 2 (`fuzz_parse_inventory`, `fuzz_parse_outcomes`) | +2 |
| Moon tasks `titania-*` | 10 | 12 | +2 (titania-kani, titania-mutants) |
| Moon composite gates | 3 (gate-edit, gate-prepush, gate-release) | 4 | +1 (gate-full) |

## No regressions in v1 lanes
- Edit / Prepush / Release lane artifacts unchanged.
- Lane receipts identical for v1.0 contracts.

## Lane outcome schema
v1.5 lanes emit the same `LaneOutcome` JSON shape. Aggregator unchanged.

## Acceptance
No regression. v1.5 scope adds the lanes; v1 contracts unaffected.
