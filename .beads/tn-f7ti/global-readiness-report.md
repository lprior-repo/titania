# Global readiness report — tn-f7ti

Mandated gates (all exit 0 unless noted; logs under evidence/):

- cargo fmt --all -- --check: exit 0.
- cargo check --workspace --all-targets --all-features: exit 0.
- Source-only strict clippy per AGENTS.md: exit 0.
- cargo test -p titania-lanes --all-features: 262 passed, 1 failed.
  The failure is dylint_reject_repair, reproduced on the untouched base
  (BLOCK_GLOBAL, pre-existing, out of scope).
- RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features
  --no-deps: exit 0.

## Deferred global lanes (accepted, out of scope)

- Full `moon ci` (moon 2.2.4 present): not run per delivery order; owned by
  tn-4isu/tn-7bq2 lanes.
- kani lanes: kani binary absent; cargo-fuzz absent.
- verus 0.2026.05.05.d03e906 and flux 4d329f2 present but not executed:
  this bead is coverage integrity, never proof of executable correctness,
  and the delivery order scopes formal execution out.

No global lane failure is introduced by this bead: the only red tests are
red on the base commit too. Workspace-wide run: titania-check
cli_dispatch shows 47 passed / 8 failed with this bead vs 47 passed /
8 failed on the untouched base (identical set: shared-worktree
.titania/out races and hermetic-env dependence), plus the base-red
dylint_reject_repair and transient /tmp quota flakes routed around with
a local TMPDIR. Evidence: evidence/workspace-tests.log.
