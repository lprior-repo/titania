# Test writer report — tn-vr2n

Written failing-first per test-plan.md, then turned green by the state-11
implementation. No test was weakened to match implementation at any point.

## Proof/Refinement Coverage Matrix

| Obligation | Files written | Scenarios | Pre-impl | Post-impl |
|---|---|---|---|---|
| tnvr2n.O-P1-BEH | `crates/titania-lanes/tests/run_cargo_invocation.rs` (new, 8 tests) | empty/bare/equals/post-separator/warn/benign/lookalike/message | E0432 compile fail (log `gate1-failing-before.log`, exit 101) | 8/8 green |
| tnvr2n.O-P1-BEH (wiring) | `run_cargo_lane.rs` tests: `run_lane_rejects_cap_lints_before_spawn` | Policy error, token named, no spawn | missing `Policy` variant (same red gate) | green |
| tnvr2n.O-P2-BEH | `run_cargo/args.rs` tests: `no_lane_argv_carries_cap_lints` | all 5 lanes scanned | missing `invocation` module (same red gate) | green |
| tnvr2n.O-P3-BEH | env suite +2, config suite +1, scrub suite +1, CLI suite +1 | ceiling values per channel | green before and after (locks, not gap) | green |

New tests total: 8 integration + 2 unit + 5 regressions = 15 scenarios,
all with exact assertions (no `is_ok()`-only checks; accept paths assert
`== Ok(())`, which required `PartialEq` on the error type by design).
