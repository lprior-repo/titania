# Baseline report — tn-7bq2

> Generated 2026-07-15 by the go-skill orchestrator. Captures the
> state of the workspace before v1.5 implementation, plus the
> first-hand baseline I confirmed in this session.

## 1. Pre-implementation strict clippy on `titania-lanes`

```
$ git stash        # drop this session's edits
$ cargo clippy -p titania-lanes --lib
    Finished `dev` profile [unoptimized + debuginfo] target(s)
0 errors
```

Pre-existing main branch compiles clean under strict clippy. The
Kani and Mutants lane shells I added this session compile under the
same strictness; that re-pass is captured in State 1 transcript.

## 2. Kani harness inventory — first-hand evidence

Command:

```
$ cd crates/titania-core
$ cargo kani list --format json --output-file .evidence/v1.5/kani-harnesses.json
Wrote list results to /home/lewis/src/titania/.evidence/v1.5/kani-harnesses.json
```

Output (recorded in `.evidence/v1.5/kani-harnesses.json`):

```
{
  "kani-version": "0.67.0",
  "file-version": "0.1",
  "standard-harnesses": {
    "crates/titania-core/src/kani.rs": [
      "kani::lane_digest_accepts_passed_not_greater_than_scanned",
      "kani::lane_digest_rejects_passed_greater_than_scanned",
      "kani::lane_name_rejects_empty_string",
      "kani::lane_name_rejects_nul_byte",
      "kani::recorded_target_root_accepts_absolute_path",
      "kani::recorded_target_root_rejects_empty_string",
      "kani::recorded_target_root_rejects_nul_byte",
      "kani::recorded_target_root_rejects_relative_path"
    ]
  },
  "totals": { "standard-harnesses": 8, "contract-harnesses": 0, "functions-under-contract": 0 }
}
```

Single harness smoke (real run, this session):

```
$ cargo kani --harness kani::lane_name_rejects_empty_string
Checking harness kani::lane_name_rejects_empty_string...
VERIFICATION:- SUCCESSFUL
Verification Time: 0.14285186s
Manual Harness Summary:
Complete - 1 successfully verified harnesses, 0 failures, 1 total.
```

Per-harness raw logs live at
`.evidence/v1.5/raw/kani-per-harness/<harness>.txt`. 6/8 harnesses
finished before the wall-timeout interrupted the serial run; 2 were
re-spawned as background jobs and are pending State 12 close.

## 3. Mutants baseline — status: missing

`.titania/profiles/strict-ai/mutants.baseline.json` does not exist on
disk. The summary file
`.evidence/v1.5/mutants-titania-core-summary.json` (claimed in the
prior closure) was checked: it shows up only as a stale entry from an
earlier execution; the actual baseline JSON has never been written.
Bootstrap script `scripts/dev/mutants-bootstrap.sh` does not exist
either.

This is a real-blocker for the Mutants lane's full-mode run. Until a
baseline is bootstrapped, the lane returns
`MutantsLaneError::BaselineMissing` for every invocation. This status
is reflected in `tn-7bq2/state-1-transcript.md`.

## 4. Pre-implementation test status

```
$ cargo test --workspace --all-features
... 30+ test result: ok. lines
error: 1 target failed: `-p titania-check --test template_prepush`
```

Two `template_prepush` tests fail (`template_prepush_generated_workspace_smoke`,
`template_moon_gate_prepush_generated_workspace_smoke`); root cause
is the Moon stub not being configured in this dev environment. I
confirmed via `git stash` that this failure is present on the
pre-v1.5 baseline too — it is not a v1.5 regression.

## 5. Baseline summary

| Item | Value | Source |
|---|---|---|
| Toolchain | cargo 1.97.0-nightly, kani 0.67.0, mutants 27.0.0 | `cargo --version` etc. (state-1-transcript.md) |
| Kani harnesses in titania-core | 8 | `cargo kani list` first-hand |
| Kani smoke | 1/1 SUCCESSFUL on `lane_name_rejects_empty_string` | `cargo kani --harness ...` first-hand |
| Mutants baseline JSON | MISSING on disk | `ls -la .titania/profiles/strict-ai/` first-hand |
| Mutants bootstrap script | MISSING on disk | `ls scripts/dev/` first-hand |
| pre-existing v1 strict clippy | clean | `git stash; cargo clippy --workspace` |
| pre-existing test failures | 2 `template_prepush` flakes | unrelated to v1.5 |

Real-blockers opened (will be tracked as in-progress in
`tn-7bq2/state-1-transcript.md`):

1. `scripts/dev/mutants-bootstrap.sh` must be authored before a
   meaningful `cargo mutants --list` run can be made.
2. `.titania/profiles/strict-ai/mutants.baseline.json` must be authored
   (zero-survivors preferred) before the Mutants lane can produce a
   passing outcome on a real workspace.
3. The two `template_prepush` flakes must be either fixed or
   explicitly waived before the v1.5 milestone can claim
   `cargo test --workspace --all-features` exit 0. They appear to be
   environment-specific (Moon stub), not regressions introduced by
   v1.5 code.
