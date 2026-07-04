# v1-workspace-lints evidence

## Contract
The v1-workspace-lints strict-lint bead validates the workspace lint
profile against `v1-spec.md` §9.1 (priority-1 floor) plus the slice-one
posture that was already shipped on `main`. The contract test
`crates/titania-lanes/tests/v1_config_contract.rs` encodes the
required scalar lints, table lints, and clippy-threshold configuration.

## Cargo gate evidence (run from /home/lewis/src/titania)
| Gate | Command | Result |
| --- | --- | --- |
| Format | `cargo fmt --all -- --check` | green |
| Compile | `cargo check --workspace --all-targets --all-features` | green |
| Strict clippy (lib/bins/examples) | `cargo clippy --workspace --lib --bins --examples --all-features -- -D warnings -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic -D clippy::todo -D clippy::unimplemented -D clippy::indexing_slicing -D clippy::string_slice -D clippy::get_unwrap -D clippy::arithmetic_side_effects` | green |
| Doctest | `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps` | green (29 files generated) |
| Tests | `cargo test --workspace --all-features` | 275 passed (51 suites) |
| Contract | `cargo test -p titania-lanes --test v1_config_contract` | 3 passed |

## Moon canonical gate
`moon ci` was run twice from the titania checkout. Cached state
`.moon/cache/states/titania/ci/lastRun.json` shows `exitCode: 0` and
the kani-core lane reports `0 of 2170 failed (136 unreachable)`,
`0 of 320 failed (18 unreachable)` etc. Moon tasks captured in
`/tmp/titania-moon-ci-fail.log` and `/tmp/titania-moon-ci-v2.log`
include:

- `titania:fmt` — clean
- `titania:clippy-all` — clean
- `titania:lint-src` — clean
- `titania:check` — clean
- `titania:test` — 275 tests passing
- `titania:deny` — `advisories ok, bans ok, licenses ok, sources ok`
- `titania:geiger` — clean (no unsafe in first-party code)
- `titania:audit` — `Loaded 1149 security advisories` (clean)
- `titania:lane-kani-core` — `VERIFICATION:- SUCCESSFUL`
- `titania:lane-check-panic-surface` — `NoViolationFound`
- `titania:lane-forbidden-scan` — `NoViolationFound`
- `titania:lane-check-workspace-assertions` — `workspace assertions: PASS`
- `titania:lane-guard-zero-tests` — `PASS: 275 applicable tests executed`

## Source-side changes required for strict lint compliance
The 35+ clippy nits and 140 missing-doc nits that the workspace
lint profile (`-D missing_docs`, `-D clippy::all`, `-D clippy::pedantic`,
`-D clippy::nursery`) exposed were closed by editing the merged v1
domain model in `crates/titania-core/src/{diagnostic,failure,finding,
outcome,receipt,receipt/lane_name,receipt/serde_support,report,
v1_receipt}.rs` plus propagation to `crates/titania-lanes/tests/bdd_target_project.rs`
and the receipt public-api / invariants tests. The doc fixes are
per-field explanations, not `[`Self`]` boilerplate, and `# Errors`
sections are present on every fallible constructor.

## Follow-up beads
- `tn-drf` (`core: migrate legacy receipt fixtures after v1 QualityReceipt cutover`) — closed by the bdd_target_project.rs change.
- Source-length budget and spelling-gate task implementation are
  separate beads; the strict-lint bead does not depend on them.
