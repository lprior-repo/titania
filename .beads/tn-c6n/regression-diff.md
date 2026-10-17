# Regression diff — tn-c6n

Baseline (State 1, before this bead): `receipt_invariants` 9/9 green; no
`receipt_domain_*` targets; no recorded verifier output.

After (State 12, production unmodified):

- `receipt_invariants` (pre-existing): 9 passed, 0 failed — unchanged.
- `receipt_domain_invariants` (new): 14 passed, 0 failed.
- `receipt_domain_properties` (new): 6 passed, 0 failed.
- Full `cargo test -p titania-core --all-features`: 30 test binaries, every
  one `ok`, zero failures (evidence/test-core-s12.log).
- Strict source-only clippy, workspace check, fmt, and rustdoc all exit 0,
  so no regressions were introduced anywhere in the workspace.

Production diff: empty (`git status` shows only the two new untracked test
files under `crates/`). There is no behavior regression by construction —
and the suite confirms the pre-existing tests still pass unchanged.
