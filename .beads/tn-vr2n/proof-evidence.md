# Proof evidence — tn-vr2n

Behavior obligations tnvr2n.O-P1-BEH, tnvr2n.O-P2-BEH, tnvr2n.O-P3-BEH are
discharged by executed `cargo test` suites with exact assertions; raw logs
under `.beads/tn-vr2n/logs/`, ledger rows in `verification-ledger.jsonl`.

- O-P1: `cargo test -p titania-lanes --test run_cargo_invocation`
  (8 scenarios) + wiring unit test `run_lane_rejects_cap_lints_before_spawn`.
- O-P2: `cargo test -p titania-lanes --lib run_cargo`
  (`no_lane_argv_carries_cap_lints` + existing argv pin tests).
- O-P3: `cargo test -p titania-lanes --test policy_scan_env_vars
  --test policy_scan_cargo_config --test command_env_scrub`
  plus `cargo test -p titania-check --test cli_dispatch
  run_lane_rejects_extra_cap_lints_args`.

No deductive artifacts exist, so there is nothing here that could vacuously
pass: every claim above names an executed command, a log file, and a ledger
row with exit status 0.
