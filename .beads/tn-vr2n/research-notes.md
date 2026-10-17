# Research notes — tn-vr2n (all files opened, exact refs)

## 1. `crates/titania-lanes/src/policy_scan/env_vars.rs`

- Rules table (`env_rules`, lines 198-206): exactly 5 rules — `RUSTFLAGS` /
  `CARGO_ENCODED_RUSTFLAGS` (AnyNonEmptyForbidden), `RUSTC_WRAPPER` /
  `RUSTC_WORKSPACE_WRAPPER` (WrapperMustBeSccache), `RUSTC_BOOTSTRAP`
  (PresenceForbidden).
- `check_env_var` (257-267): any present value violating policy → Finding.
  There is NO rule for `CARGO_BUILD_RUSTFLAGS`, `CARGO_TARGET_*_RUSTFLAGS`,
  or `CARGO_BUILD_RUSTC_WRAPPER`. Live-bypass? No: spawn-time
  `ScrubbedEnv` drops every `CARGO_*` key except validated
  `CARGO_TARGET_DIR`/`CARGO_HOME`/`RUSTUP_HOME` (see §4). Scanner gap is
  defense-in-depth only; recorded as residual analysis, no new rules added
  (minimal-change principle).
- `is_sccache_wrapper` (277-280): accepts `sccache` or any path whose
  file_name is `sccache`. PATH shadowing is a host-environment concern
  outside the malicious-target model (target cannot set CI PATH).

## 2. `crates/titania-lanes/src/policy_scan/cargo_config.rs`

- `scan_cargo_config_from` (62-70): scans target-root `.cargo/config{,.toml}`
  + rejects ANY ancestor config (`parent_config_findings`, 79-85).
- `rustflag_finding` (211-220): EVERY `rustflags` value under `[build]` or
  any `[target.*]` (string, array, or inline table via `rustflags_values`,
  171-190) → `BYPASS_CARGO_CONFIG_FLAGS` finding. `--cap-lints` smuggled in
  config is therefore already rejected (proved by new regression test).
- `wrapper_finding` (199-209): non-`sccache` `rustc-wrapper` /
  `workspace-wrapper` → finding. Exact-match `sccache` only (stricter than
  env rule; no change needed).

## 3. `crates/titania-lanes/src/run_cargo_lane.rs`

- `cargo_output` (137-147): `CommandIn::new(target, "cargo")` +
  `inherit_env()` + `args(args_for_lane(lane))` + `args_strings(extra_args)`
  — `extra_args` NEVER validated. `run_lane` (93-103) is the sole caller;
  `run_checked` (75-81) always passes `&[]`. The signature is the hole.
- `error_exit` (48-54): Usage|Target → `Usage`; everything else → `Failure`.
  New `Policy` variant must map to `Violations` (Reject, exit 1), so a new
  match arm is required (exhaustive match forces the update — good).

## 4. `crates/titania-lanes/src/command/{env_filter,execution}.rs`

- `DEFAULT_DENY_KEYS` (env_filter 38-62) + `DEFAULT_DENY_PREFIXES =
  ["CARGO_", "BOXED_"]` (70) + allowlist (75-118).
- `build_base_command` (execution 146-161): `env_clear()` then
  `ScrubbedEnv::from_parent_for_target(...).apply_to()`. Verified:
  hostile `RUSTFLAGS=--cap-lints allow` in the parent CANNOT reach the
  cargo child (proved by new scrub regression test).
- `validated_lane_pair` (337-347): only `CARGO_TARGET_DIR` (trusted
  `.titania/cache/*`), `CARGO_HOME`/`RUSTUP_HOME` (hermetic paths) pass.
  None affects lint severity.

## 5. `crates/titania-lanes/src/run_cargo/args.rs`

- `CLIPPY_ARGS` (10-88): `clippy --workspace --lib --bins --examples
  --all-features --frozen --message-format=json -- -D warnings ...` (34
  deny pairs). No `--cap-lints` token today; exact-match unit test
  (`clippy_args_match_v1_section_9_2_exactly`) pins the argv. New unit test
  asserts the negative property (no cap-lints in ANY lane argv) so future
  edits cannot regress it.

## 6. CLI layer (`crates/titania-check/src/args/parse.rs`)

- `parse_run_lane` (387-394): any token after the lane name →
  `CliError::ExtraArgument`. `titania-check run-lane clippy -- --cap-lints
  allow` is rejected before any lane runs (proved by new CLI test).

## 7. Moon runner (`.moon/tasks/all.yml`)

- `titania-clippy` (204-205): `CARGO_TARGET_DIR=... cargo run -p titania-check
  -- run-lane clippy` → the protected invocation is exactly
  `run_cargo_lane::run(Clippy)`. Raw `cargo clippy` slice-1 tasks
  (`lint-src`, `clippy-all`) run with CI-host env, outside the
  malicious-target model.

## Patterns searched

- `cap-lints|cap_lints|CAP_LINTS` over crates/.moon/contracts/v1-spec.md:
  ZERO hits (this bead adds the first).
- `extra_args` in `titania-lanes/src`: only `run_cargo/*` + `run_cargo_lane.rs`
  (listed above); no other lane appends caller-controlled args to a compiler
  invocation.

## Duplicate beads

- tn-98g9 (policy/exception ownership) is adjacent, not overlapping.
- No other Phase-1 bead owns effective-invocation validation.

## Scope (smallest correct change)

New pure validator + chokepoint enforcement + static-argv negative test +
channel-coverage regressions. No scanner rule changes, no Moon changes, no
`--config`/`-Z` expansion (no caller can pass them; CLI rejects extras).
