# Test Plan: tn-vr2n invocation ceiling containment

## Summary

- Behaviors identified: 5 (chokepoint rejection, static-argv purity, env lock,
  config lock, scrub + CLI containment)
- Trophy allocation: 2 unit / 9 integration / 1 e2e (CLI) / static via
  existing pin tests + strict clippy gate. Deviation (integration-heavy)
  justified: the guarantee is a boundary behavior across scanners, scrub,
  CLI, and runner — pure logic is one small predicate.
- Proptest invariants: 0 new (single-input pure predicate; exhaustive
  spelling enumeration in unit/integration tests subsumes generation).
- Fuzz targets: 0 (exact-token match, no parsing boundary).
- Kani harnesses: 0 (parent-delegated exclusion, owned by tn-4isu/tn-7bq2).
- Mutation threshold target: ≥90% kill on touched code (guard call removal,
  matcher weakening, variant-remap mutations must all be caught).

## Proof/Refinement Coverage Matrix

| Obligation | Behavior tests | Refinement (second-level) checks |
|---|---|---|
| tnvr2n.O-P1-BEH | `run_cargo_invocation` suite (8 scenarios) | wiring unit test in `run_cargo_lane` (Policy, no spawn) |
| tnvr2n.O-P2-BEH | `args.rs` negative unit test (all 5 lanes) | existing exact-match pin test |
| tnvr2n.O-P3-BEH | env/config/scrub/CLI ceiling regressions | existing full suites stay green |

## 1. Behavior Inventory

1. Runner rejects ceiling-carrying extra args when guard runs before spawn.
2. Static lane argv carries no ceiling token when examined element-wise.
3. Env scanner rejects ceiling-valued RUSTFLAGS / encoded flags when scanned.
4. Config scanner rejects ceiling-valued rustflags when scanned.
5. Spawn scrub drops ceiling-valued RUSTFLAGS; CLI rejects surplus tokens.

## 2. BDD Scenarios (Given-When-Then → test fn)

### Behavior 1 (integration: `crates/titania-lanes/tests/run_cargo_invocation.rs`)

- Given empty extra args / When validated / Then `Ok(())`:
  `empty_extra_args_are_accepted`.
- Given `["--cap-lints", "allow"]` / When validated / Then
  `Err(CapLints)` naming `--cap-lints`: `bare_cap_lints_flag_is_rejected`.
- Given `["--cap-lints=allow"]` / Then `Err` naming the token:
  `equals_form_is_rejected`.
- Given `["--", "--cap-lints", "allow"]` / Then `Err`:
  `cap_lints_after_separator_is_rejected`.
- Given `["--cap-lints=warn"]` / Then `Err`: `warn_level_is_rejected`.
- Given `["--offline"]` / `["--", "--test-threads=1"]` / Then `Ok`:
  `benign_flags_are_accepted`.
- Given `["--cap-lints-allow", "cap-lints"]` / Then `Ok` (no over-block):
  `lookalike_flags_are_accepted`.
- Given rejection / Then message names token + states ceiling consequence:
  `rejection_names_offending_token`.

### Behavior 2 (unit: `run_cargo/args.rs` tests)

- Given each `CargoLane` static argv / When scanned element-wise / Then no
  element equals `--cap-lints` or opens with `--cap-lints=`:
  `no_lane_argv_carries_cap_lints`.

### Behavior 1 wiring (unit: `run_cargo_lane.rs` tests, no spawn)

- Given clippy lane + ceiling extra args / When `run_lane` runs / Then
  `Err(RunCargoError::Policy)` mentioning the token:
  `run_lane_rejects_cap_lints_before_spawn`.

### Behaviors 3–5 (regressions in existing suites)

- `rustflags_cap_lints_allow_is_a_finding`,
  `encoded_rustflags_cap_lints_allow_is_a_finding`
  (env suite; exact rule ids + messages).
- `config_rustflags_cap_lints_is_a_finding` (config suite; per-flag findings).
- `scrubbed_inherit_drops_cap_lints_rustflags` (scrub suite).
- `run_lane_rejects_extra_cap_lints_args` (CLI suite; nonzero exit).

## 3. Exact-assertion rule

No `is_ok()`-only assertions on accept paths: accept tests assert
`result == Ok(())` (requires `PartialEq` on the error type); reject tests
assert the exact variant, the exact token, and message substrings. Every
error variant of `InvocationError` (exactly one) has explicit scenarios.

## 4. Mutation Checkpoints

- Remove guard call in `run_lane` → wiring test goes red (spawn attempted).
- Weaken matcher (e.g. drop `=`-form) → `equals_form_is_rejected` red.
- Broaden matcher (substring) → `lookalike_flags_are_accepted` red.
- Remap `Policy` to `Usage` → wiring test red (expects `Policy`).
- Add ceiling token to any static argv → negative unit test red.

## Open Questions

None. Contract C1–C3 and research-notes §§1–7 fully determine the tests.
