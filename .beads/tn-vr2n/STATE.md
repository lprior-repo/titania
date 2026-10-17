# STATE.md — tn-vr2n (isolated worktree delivery)

Bead: tn-vr2n — policy: verify the effective compiler invocation cannot cap mandatory lints.
Worktree: /home/lewis/src/titania/.muse/worktrees/subagent-v2-01a14623-5a8f-76e0-be4a-be4eb153aaf7-01a146d6-7fd4-7c43-85a2-6055a7e3cfce
Base: 927893f922fe2b9c658a0c1574086afe7d6a6dc6
Claim: claimed in-session (was OPEN, listed in `bd ready`, not blocked).
Policy: no commit/push/sync. No full `moon ci`. No kani/mutants lanes (owned by tn-4isu/tn-7bq2).

## Finding (research-complete)

Zero repo mentions of `cap-lints` (rg over rs/md/yml/toml). Existing coverage:

- Env RUSTFLAGS / CARGO_ENCODED_RUSTFLAGS (any non-empty) → rejected by
  `policy_scan::env_vars` AND scrubbed at spawn by `ScrubbedEnv`
  (`command/execution.rs` `build_base_command`: `env_clear` + allowlist).
- `.cargo/config` any `rustflags` (build + every `[target.*]`) → rejected by
  `policy_scan::cargo_config`; parent-dir configs rejected outright.
- Non-sccache RUSTC_WRAPPER / config wrappers → rejected + scrubbed.
- `titania-check run-lane` CLI rejects all extra args
  (`parse_run_lane` → `extra_arg` error); `run_checked` passes `&[]`.

Gap (this bead): the library chokepoint `run_cargo_lane::run_lane` /
`cargo_output` appends `extra_args` to the protected argv with NO
validation. `cargo clippy -- --cap-lints allow` forwards the flag to the
compiler, where it ceilings every `-D` deny (cargo itself relies on this
for dependencies). A non-empty `extra_args` (future caller, API misuse)
would silently neuter the §9.2 deny argv while still reporting Clean.

## Change (minimal)

1. New pure public module `policy_scan::invocation`: `validate_extra_args`
   rejects any `--cap-lints` / `--cap-lints=*` token with typed
   `InvocationError::CapLints { arg }`.
2. Enforce at chokepoint: `run_lane` rejects before spawn; new
   `RunCargoError::Policy` → `LaneExit::Violations` (Reject, exit 1).
3. Unit-test static argv in `run_cargo/args.rs` tests (no cap-lints in any lane).
4. Regression tests proving existing channels: env RUSTFLAGS/CARGO_ENCODED_RUSTFLAGS
   `--cap-lints allow` → findings; config rustflags `--cap-lints` → findings;
   scrub drops hostile RUSTFLAGS; CLI `run-lane clippy -- --cap-lints allow` →
   CLI error, nonzero exit.

Explicitly NOT doing: invocation-count lint (bead notes forbid); new
CARGO_BUILD_* scanner rules (execution scrub already neutralizes; recorded
as residual analysis, not a live bypass); moon task changes.

## Go-skill states

1 provenance/baseline — done (validator STATUS recorded in verification-ledger).
2 explore — done (research-notes).
3 rust-contract — contract artifacts (lean: contract.md + proof-seeds.jsonl).
4 proof-planner + 4b review — lane decisions: implementation-bound lanes only
  (unit/behavior, no Verus/Kani per parent lane exclusion + pure-sync code).
5 proof-writer — none (no proof obligations; rationale recorded).
6 proof-reviewer — self-review recorded; independent review PENDING (residual).
7 bridge — proof-to-implementation map (tests ↔ source refs).
8/9/10 test plan/write/review — failing-first, then green.
11 holzman-rust implementation.
12 formal-verifier — targeted gates (fmt/check/clippy-src/test-touched/doc).
13 black-hat review — self-attack notes; independent review PENDING.
14 evidence-packaging + truth-serum self-audit.
15 landing — NOT landing (no commit/push/sync authorized); handoff only.
16 final verify + report.
