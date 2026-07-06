# tn-pdn Evidence Bundle — Killer demo delivered

Date: 2026-07-05
Workspace: `/home/lewis/src/titania/.worktrees/v1-combined-dispatch`
Bead: `tn-pdn` — demo: prove for-loop plus unwrap rejection

## Requirement mapping

| Requirement | Source / behavior | Evidence |
|---|---|---|
| Bad fixture rejects with `FUNC_LOOPS_FOR` and `CLIPPY_UNWRAP_USED` | `fixtures/strict_ai_loop_unwrap/bad/src/lib.rs` contains one `for` loop and one `.unwrap()` | `cargo test -p titania-check --test killer_demo` passed 15/15; raw direct stdout saved at `.beads/tn-pdn/raw/bad-stdout.json` |
| Bad fixture has empty `gate_failures` | `titania-check --scope edit --emit json` runs all edit lanes, then aggregates typed artifacts | `.beads/tn-pdn/raw/direct-cli-summary.json`: bad exit 1, `variant=reject`, `gate_failures=0` |
| Repaired fixture passes with schema_version 1 receipt | `fixtures/strict_ai_loop_unwrap/repaired/src/lib.rs` uses a non-panicking iterator pipeline | `.beads/tn-pdn/raw/direct-cli-summary.json`: repaired exit 0, `variant=pass`, `receipt_schema=1` |
| All seven edit lanes are represented | `GateScope::Edit` lanes: Fmt, Compile, Clippy, AstGrep, Dylint, PanicScan, PolicyScan | Both direct reports saved under `.beads/tn-pdn/raw/` include all seven `per_lane` entries |
| No Dylint infra false failure in acceptance path | Acceptance path uses the same tool PATH as `cargo test`, where `/cache/cargo-shared/bin/cargo-dylint` is available | `strace -f -e execve cargo test -p titania-check --test killer_demo repaired_fixture_passes_with_receipt -- --exact` showed `CARGO_BIN_EXE_titania-check=/home/lewis/src/titania/.worktrees/v1-combined-dispatch/target/debug/titania-check` and `execve("/cache/cargo-shared/bin/cargo-dylint", ["cargo-dylint", "--version"], ...) = 0` |

## Command evidence

### Rust checks

- `cargo fmt --all -- --check` exited 0.
- `cargo check -p titania-core -p titania-aggregate -p titania-check --all-targets` exited 0.
- `cargo clippy -p titania-core -p titania-aggregate --lib --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` exited 0.
- `cargo clippy -p titania-check --bins --examples --all-features -- -D warnings -D unsafe_code -D clippy::unwrap_used -D clippy::expect_used -D clippy::panic` exited 0.
- `cargo test -p titania-core` passed 169 tests.
- `cargo test -p titania-aggregate` passed 33 tests.
- `cargo test -p titania-check --test aggregate_cli` passed 3 tests.
- `cargo test -p titania-check --test cli_dispatch` passed 23 tests.
- `cargo test -p titania-check --test killer_demo` passed 15 tests.

### Final canonical gate after scanner cleanup

- `cargo test --workspace --all-features --frozen -- --test-threads=1` exited 0: 627 tests passed across 95 suites.
- `moon ci --force --summary normal` exited 0: 57 actions completed, 2 skipped; edit, prepush, and release gates all emitted `variant="pass"` receipts.
- Targeted forbidden-scan regression checks after cleanup:
  - `cargo test --bin forbidden-scan` exited 0: 21 tests passed.
  - `cargo clippy --bin forbidden-scan` exited 0.
  - `moon run titania:clippy-all` exited 0.
  - `moon run titania:lint-src` exited 0.
  - `moon run titania:lane-forbidden-scan` exited 0 with `NoViolationFound`.
  - `moon run titania:lane-check-source-length` exited 0 with `NotApplicable: legacy compile split directory absent`.
  - `moon run titania:fmt` exited 0.
  - Raw session outputs: `artifact://2302`, `artifact://2304`, `artifact://2311`, `artifact://2312`, `artifact://2313`.

### Direct CLI acceptance, fresh temp fixtures

Direct CLI evidence was run against fresh temp copies outside the repo tree to avoid parent `clippy.toml` contamination. Environment explicitly prepended `/cache/cargo-shared/bin` so `cargo-dylint` matched the `cargo test` execution PATH.

- Bad temp fixture: `/tmp/titania-pdn-bad-x2mvq20w`
  - Command: `target/debug/titania-check --scope edit --emit json`
  - Exit: 1 (expected `Report::Reject`)
  - Stderr: empty
  - JSON: `.beads/tn-pdn/raw/bad-stdout.json`
  - Summary: `variant=reject`, `code_findings=[CLIPPY_UNWRAP_USED,FUNC_LOOPS_FOR]`, `gate_failures=0`, all seven edit lanes present.
- Repaired temp fixture: `/tmp/titania-pdn-repaired-azhxjbdm`
  - Command: `target/debug/titania-check --scope edit --emit json`
  - Exit: 0
  - Stderr: empty
  - JSON: `.beads/tn-pdn/raw/repaired-stdout.json`
  - Summary: `variant=pass`, `receipt.schema_version=1`, `gate_failures=0`, all seven edit lanes present.

## Contaminated diagnostic intentionally excluded from acceptance

Running the bad fixture in place under `.worktrees/v1-combined-dispatch/fixtures/...` sees the repository parent `clippy.toml`, so Clippy also emits `CLIPPY_DISALLOWED_METHODS`. That diagnostic is real for an in-repo path but is not the independent fixture contract. The accepted path copies the fixture to a fresh temp workspace, matching `killer_demo.rs` and avoiding parent policy leakage.

Running the repaired fixture without `/cache/cargo-shared/bin` in `PATH` produces a Dylint infra gate failure because `cargo-dylint` is absent from the default shell PATH. That environment mismatch is not an implementation failure; cargo-test evidence and direct temp CLI evidence both pass when `/cache/cargo-shared/bin` is available.

## Residual blockers

None for `tn-pdn`. Broader `tn-fqd` thin-executor work remains a separate P1 public-UX bead, not a blocker for this killer-demo acceptance path.
