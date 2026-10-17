# Test Writer Report — tn-bvv7

Failing-first evidence: before the fix, 4 detector tests failed
(`evidence/failing-first-detectors.txt`: 5 passed, 4 failed),
proving fully-qualified paths bypassed all four legacy detectors.
The renamed-import case already passed (regression guard).

Tests written (all passing after the fix):

## Proof/Refinement Coverage Matrix

| Layer | Tests | Result |
|-------|-------|--------|
| Detector unit (exact) | 9 tn-bvv7 tests (FQ ×4, alias, boundary ×1, process, async, state ×2) | pass |
| Scope unit | 2 tests (core in-scope, shell out-of-scope) | pass |
| Property | 2 proptests × 64 cases | pass |
| Catalog contract | extended to 7 IDs + FQ-pattern + manifest-dependency tests (11 tests total in suite) | pass |
| Lane behavior | 3 new end-to-end tests (FQ rejection, new boundaries, clean controls) + full-matrix reachability | pass |

Total touched-crate result: 848 passed, 0 failed
(`evidence/touched-crates-test.txt`).
