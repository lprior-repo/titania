# v1.5 Baseline Report

## Tools
- `cargo` (workspace pinned via rust-toolchain.toml): nightly-2026-04-27
- `cargo-kani` 0.67.0 (installed)
- `cargo-mutants` 27.0.0 (installed)
- `systemd-run` (host cgroup support; required for Kani lane)

## Pre-impl Evidence (already collected)
- `.evidence/v1.5/kani-harnesses.json` — 8 standard harnesses confirmed
- `.evidence/v1.5/raw/kani-single-harness-smoke.txt` — VERIFICATION:- SUCCESSFUL on `kani::lane_name_rejects_empty_string`
- `.evidence/v1.5/raw/kani-list-stdout.txt` — `cargo kani list --package` rejected in 0.67.0; per-crate fallback documented
- `.evidence/v1.5/mutants-titania-core-summary.json` — 480 mutants generated, 236 build-survivors (cargo mutants --check); full test-mode bootstrap deferred

## Baseline Gates
- `cargo check --workspace --all-targets` — green
- `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` — green
- `cargo test --workspace --all-features` — green
- `cargo doc --workspace --no-deps -D warnings` — TBD (run before holzman-rust impl)

## Open
- 9 production match sites need Lane::{Kani,Mutants} + GateScope::Full arms (per `.evidence/v1.5/spec.md` §11).
- `.titania/profiles/strict-ai/mutants.baseline.json` does not exist yet; bootstrap is part of tn-7bq2.4.
- bd memory keys for v1.5: `v15-kani-inventory`, `v15-mutants-titania-core`, `v15-contract-emitted` (saved).
