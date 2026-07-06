# tn-d3s.3 Release Pass Evidence

Date: 2026-07-05
Workspace: `/home/lewis/src/titania/.worktrees/v1-combined-dispatch`
Bead: `tn-d3s.3` - dogfood: prove own repo release pass after repair children close

## Commands and results

### Release scope through current workspace build

```bash
cargo run --quiet -p titania-check -- --scope release --emit json
```

Result: exit 0. Raw output: `artifact://2388`.

Recorded JSON: `.evidence/dogfood/release-pass.json`.

Observed release report:

- `variant`: `pass`
- `scope`: `Release`
- clean lanes: 10/10
- lanes: `Fmt`, `Compile`, `Clippy`, `AstGrep`, `Dylint`, `PanicScan`, `PolicyScan`, `Test`, `Deny`, `Build`
- `source_digest`: `2c508e5728ec37b591ef5ed0f2d9c664182ea0fe802ad90e0ff11e4995b16a40`
- `policy_digest`: `a368c104f20386ad665b5c70a1db7a5cca9948d72951cb532c89c3f09dd47761`
- `toolchain_digest`: `2f12da5e40c3e6cf4b0a45eee548c1b8bb22ee21b90d47939821a57e74dada84`

This command uses Cargo from the current worktree, so the `titania-check` binary is built/resolved from `/home/lewis/src/titania/.worktrees/v1-combined-dispatch`, not from an installed PATH binary.

### Required workspace tests

```bash
cargo test --workspace --all-features
```

Result: exit 0. Summary: 632 tests passed across 95 suites. Raw output: `artifact://2385`.

### Canonical Moon CI gate

```bash
moon ci --force --summary normal
```

Result: exit 0. Summary: 57 actions completed, 2 skipped; `titania:gate-release` passed. Raw output: `artifact://2387`; full raw output: `artifact://2386`.

Relevant terminal summary:

- `pass RunTask(titania:gate-edit)`
- `pass RunTask(titania:gate-prepush)`
- `pass RunTask(titania:gate-release)`
- `Actions: 57 completed, 2 skipped`

## Dependency/repair status

- `tn-d3s.1` closed after release dogfood manifest captured a pass with zero findings.
- `tn-d3s.2` closed after reviewed self-exception tests and policy-scan expiry path proof landed.
- No source edits were made for `tn-d3s.3`; this bead is evidence-only.

## Residual risk

None observed for `tn-d3s.3`: the release scope passed through the current workspace build, the workspace test suite passed, and canonical Moon CI passed.
