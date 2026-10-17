# Global readiness — tn-c6n

Canonical CI is Moon (`moon ci`). Per delegation, the full `moon ci` lane is
out of scope for this bead (pre-existing, owned elsewhere); it is not executed
here and its result is not claimed.

Global-state risks noted, not owned:

- `kani` is absent on this host, so no kani lane can execute here regardless of
  scope. Bounded-proof ownership sits with `tn-7bq2` per delegation.
- `verus` 0.2026.05.05.d03e906 is present and verifies the registered schema
  binding file (see baseline).
- Working tree is clean at base `927893f`; no other worker files are touched
  (this worktree is the only write target).

Targeted gates owned by this bead (State 12): `cargo fmt --check`,
`cargo check --workspace --all-targets --all-features`, source-only strict
clippy per `AGENTS.md`, `cargo test` for touched crates (fully green),
`RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps`.
