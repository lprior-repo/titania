# Machine gate report — tn-f7ti

writer_skill: formal-verifier
writer_invocation_id: INV-14-formal-verifier

All commands run from the isolated worktree; full logs under evidence/.

- cargo fmt --all -- --check → exit 0 (evidence/fmt-check.log, empty).
- cargo check --workspace --all-targets --all-features → exit 0
  (evidence/check.log).
- Source-only strict clippy per AGENTS.md (workspace lib+bins+examples,
  all features, full deny set) → exit 0 (evidence/clippy-src.log).
- cargo test -p titania-lanes --test proof_registry --all-features →
  exit 0, 13 passed (evidence/proof_registry-tests.log).
- cargo test -p titania-lanes --all-features → 262 passed, 1 failed
  (evidence/titania-lanes-tests.log). The failure is
  dylint_reject_repair, reproduced on the untouched base commit;
  classified pre-existing/environmental and out of bead scope.
- RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features
  --no-deps → exit 0 (evidence/doc.log).

Toolchain: cargo 1.97.0-nightly (eb9b60f1f 2026-04-24),
rustc 1.97.0-nightly (ca9a134e0 2026-04-26).

Workspace-wide note: cargo test --workspace shows only failures that
reproduce identically on the untouched base (titania-check cli_dispatch
47/8 both ways; dylint_reject_repair base-red). Full log:
evidence/workspace-tests.log.
