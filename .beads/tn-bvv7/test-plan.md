# Test Plan — tn-bvv7

Behavior-test strategy (Testing Trophy: integration-heavy, exact
assertions, deterministic, public-API surface):

## Proof/Refinement Coverage Matrix

| Requirement | Behavior tests | Property harness | Gate |
|-------------|----------------|------------------|------|
| REQ-3 renamed/FQ/macro-hidden effects block | `ast_grep_architecture` fixture-content + catalog tests (11 tests); `ast_grep_lane` FQ rejection test; 9 detector unit tests | `tn_bvv7_fq_bypass_property` (64 cases) | cargo test exit 0 |
| REQ-1 process/async/state block | detector tests (process/async/state/allowance); lane new-boundaries test; manifest dependency test | `tn_bvv7_effect_boundary_property` (64 cases) | cargo test exit 0 |
| REQ-2 happy paths pass | allowed fixtures content tests; scope in/out tests; lane clean-controls test | clean-padding control inside both properties | cargo test exit 0 |

No `is_ok()`-only assertions: every test asserts exact rule IDs,
exact file content, or exact boolean outcomes on named inputs.
Verifier harnesses (property tests) never substitute for the
integration behavior tests; both run in the same `cargo test`
invocation with exit 0.
