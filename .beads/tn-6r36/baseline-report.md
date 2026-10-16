# Baseline Report — tn-6r36

Scope is contract-only; no production code changes. Baseline pins the
pre-existing enforcement behavior that the contract redefines:

- `crates/titania-dylint/src/functional_surface.rs` sha256 14090a44:
  rejects all for/while/loop and all unwrap_or* by spelling; skips
  from_expansion spans; late method pass has no test-module gate.
- `crates/titania-policy/src/exceptions.rs` sha256 e0c6df6b: strict
  exceptions.toml shape (deny_unknown_fields parity, PolicyDate ordering).
- `clippy.toml`: bans unwrap/expect/unwrap_or families, panic/todo macros,
  production asserts; strict thresholds (complexity 8, nesting 2, lines 60).
- `Cargo.toml` workspace lints: deny warnings/pedantic/nursery, panic-free
  production, no indexing/slicing/casts/arithmetic side effects.

No behavior baseline (tests/benches) is altered by this bead. Follow-up
implementation beads diff against these hashes.
