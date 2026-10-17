# Proof/test/source alignment — tn-vr2n

| Requirement | Proof ID | Refinement ID | Source Refs | Behavior Test Refs | Refinement Harness Refs | Commands Run | Ledger Result | Status |
|---|---|---|---|---|---|---|---|---|
| tnvr2n-REQ-CHOKEPOINT | tnvr2n.O-P1-BEH | tnvr2n.B-P1 | titania-lanes/src/policy_scan/invocation.rs::validate_extra_args, titania-lanes/src/run_cargo_lane.rs::run_lane | crates/titania-lanes/tests/run_cargo_invocation.rs | crates/titania-lanes/src/run_cargo_lane.rs | cargo test -p titania-lanes --test run_cargo_invocation | PASS | closed |
| tnvr2n-REQ-STATIC-ARGV | tnvr2n.O-P2-BEH | tnvr2n.B-P2 | titania-lanes/src/run_cargo/args.rs::args_for_lane | crates/titania-lanes/src/run_cargo/args.rs | crates/titania-lanes/tests/run_cargo_invocation.rs | cargo test -p titania-lanes --lib run_cargo | PASS | closed |
| tnvr2n-REQ-ESTABLISHED-LOCKS | tnvr2n.O-P3-BEH | tnvr2n.B-P3 | titania-lanes/src/policy_scan.rs::scan_policy_inputs_with_exceptions | crates/titania-lanes/tests/policy_scan_env_vars.rs, crates/titania-lanes/tests/policy_scan_cargo_config.rs, crates/titania-lanes/tests/command_env_scrub.rs, crates/titania-check/tests/cli_dispatch.rs | crates/titania-lanes/tests/run_cargo_invocation.rs | cargo test -p titania-lanes (channel suites) + cli_dispatch | PASS | closed |
