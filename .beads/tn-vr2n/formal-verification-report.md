# Formal verification report — tn-vr2n

STATUS: PASS

All three required behavior obligations discharged by executed commands
(exit 0, raw logs retained):

- tnvr2n.O-P1-BEH: `cargo test -p titania-lanes --test run_cargo_invocation`
  → 8/8 green; plus lib wiring test in the same lanes run.
- tnvr2n.O-P2-BEH: `cargo test -p titania-lanes --lib run_cargo` → green
  within the full lanes run.
- tnvr2n.O-P3-BEH: env/config/scrub suites green within the full lanes run;
  `cargo test -p titania-check --test cli_dispatch` → 56/56 green.

Ledger rows: verification-ledger.jsonl (6 PASS rows, status closed).
No waivers consumed. Excluded lanes (kani/mutants/moon ci) are recorded as
residuals, not passes.
