# tn-d3s.1 Release Dogfood Audit

Date: 2026-07-05
Workspace: `/home/lewis/src/titania/.worktrees/v1-combined-dispatch`
Bead: `tn-d3s.1` — dogfood: capture current release-scope finding manifest

## Commands

1. Exact command without PATH preparation:

```bash
titania-check --scope release --emit json
```

Result: exit 127, `command not found: titania-check`.

2. Acceptance command with the built workspace binary and cargo tools on PATH:

```bash
PATH=/home/lewis/src/titania/.worktrees/v1-combined-dispatch/target/debug:/cache/cargo-shared/bin:$PATH \
  titania-check --scope release --emit json
```

Result: exit 0. Raw output: `artifact://2322`.

## Release-scope outcome

- Report variant: `pass`.
- Scope: `Release`.
- Finding count: 0.
- Gate failure count: 0.
- Per-lane entries: 10.
- Lanes present and clean: `Fmt`, `Compile`, `Clippy`, `AstGrep`, `Dylint`, `PanicScan`, `PolicyScan`, `Test`, `Deny`, `Build`.
- Receipt digests present: `source_digest`, `cargo_lock_digest`, `policy_digest`, `toolchain_digest`.

## Manifest

The machine-readable manifest is `.evidence/dogfood/release-finding-manifest.jsonl`.

Because release scope passed with zero findings, no repair child beads are proposed by `tn-d3s.1`.

## Residual risk

The unqualified command is not available on the default shell PATH in this session. Evidence used the workspace-built binary by prepending `target/debug` and `/cache/cargo-shared/bin` to PATH. This is an invocation-environment issue, not a release-scope finding.
