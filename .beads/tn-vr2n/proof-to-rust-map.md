# Proof-to-Rust map — tn-vr2n

| Proof ID | Claim | Behavior Affecting | Rust Source Refs | Behavior Test Refs | Refinement Harness Refs | Verifier | Evidence Command | Rerun From |
|---|---|---|---|---|---|---|---|---|
| tnvr2n.O-P1-BEH | extra-arg ceiling rejected at chokepoint | true | titania-lanes/src/policy_scan/invocation.rs::validate_extra_args, titania-lanes/src/run_cargo_lane.rs::run_lane | crates/titania-lanes/tests/run_cargo_invocation.rs | crates/titania-lanes/src/run_cargo_lane.rs (wiring unit test) | proptest | cargo test -p titania-lanes --test run_cargo_invocation | 4 |
| tnvr2n.O-P2-BEH | static argv ceiling-free | true | titania-lanes/src/run_cargo/args.rs::args_for_lane | crates/titania-lanes/src/run_cargo/args.rs (negative unit test) | crates/titania-lanes/tests/run_cargo_invocation.rs | proptest | cargo test -p titania-lanes --lib run_cargo | 4 |
| tnvr2n.O-P3-BEH | established path locks hold for ceiling values | true | titania-lanes/src/policy_scan.rs::scan_policy_inputs_with_exceptions | crates/titania-lanes/tests/policy_scan_env_vars.rs, crates/titania-lanes/tests/policy_scan_cargo_config.rs, crates/titania-lanes/tests/command_env_scrub.rs, crates/titania-check/tests/cli_dispatch.rs | crates/titania-lanes/tests/run_cargo_invocation.rs | proptest | cargo test -p titania-lanes --test policy_scan_env_vars --test policy_scan_cargo_config --test command_env_scrub | 4 |

Bridge discipline: behavior tests call production APIs directly (no copied
models, no shadow types); refinement refs are second-level executable
checks over the same production code, disjoint files from the behavior refs.
