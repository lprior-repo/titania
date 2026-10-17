# Test plan — tn-4isu (State 8)

## Proof/Refinement Coverage Matrix

| Proof/Refinement ID | Behavior claim | Behavior test | Result |
|---|---|---|---|
| tn4isu.OBL-DIFF / RRO-DIFF | none (proof-only) | existing titania-core suite guards constructors (path_range_unit_tests, 35 tests) | green, evidence/cargo-test-core.log |
| tn4isu.OBL-WIRE / RRO-WIRE | none (proof-only) | existing `tests/outcome_serde.rs` (22 tests) guards LaneOutcome serde behavior | green, evidence/cargo-test-core.log |
| tn4isu.OBL-ID / RRO-ID | none (proof-only) | existing unit tests (8) guard proof-id constructors | green, evidence/cargo-test-core.log |
| tn4isu.OBL-FULL / RRO-FULL | none (proof-only) | full titania-core suite: 8+5+4+22+22+35 = 96 tests green | green, evidence/cargo-test-core.log |

## Failing-first analysis

No new failing-first behavior test is warrantable: the change alters unwind
attributes on `cfg(kani)` harnesses, which `cargo test` does not compile and no
consumer can observe. Writing a behavior test that cannot fail would be proof
theater (bead §7.5 anti-hallucination guard). The honest behavior strategy:

1. Existing suite stays green (done: exit 0, 96/96).
2. New-behavior tests: none required (no new behavior).
3. Regression tests for the failure mode (solver exhaustion): the Kani harness
   verdicts themselves at State 12 — verifier harnesses are not behavior tests
   and are not presented as such.

Negative/positive control: `outcome_serde.rs` round-trip tests fail on any serde
behavior change (paired control for the wire-harness domain); they pass here,
confirming zero behavior drift.
