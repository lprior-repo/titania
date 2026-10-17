# Regression diff — tn-vr2n

Baseline (pre-change): `policy_scan_env_vars`, `policy_scan_cargo_config`,
`command_env_scrub` suites green (log `baseline-suites.log`, 8+18+16 tests).

Post-change: full `titania-lanes` suite green with zero failures across all
30 targets (log `gate-test-lanes.log`); full `cli_dispatch` suite green
56/56 (in `gate-test-check.log`).

Delta attributable to this bead: +15 scenarios (8 invocation + 2 unit +
5 channel regressions), all green. No previously-green test changed
outcome: every pre-existing test file diff is purely additive (verified via
`git diff`: removed lines are only fmt reflows of added lines).

No regressions. One pre-existing environmental failure lives outside the
touched surface (`template_prepush` smoke tests, base-reproduced).
