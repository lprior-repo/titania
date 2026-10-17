# Global readiness — tn-vr2n

Per parent delegation, full `moon ci` is NOT run in this delivery
(pre-existing global state owned by tn-4isu/tn-7bq2; running it here would
duplicate that lane and risk attributing unrelated failures to this bead).

Substituted targeted gates (mandatory, all must exit 0):

- `cargo fmt --all -- --check`
- `cargo check --workspace --all-targets --all-features`
- source-only strict clippy per AGENTS.md (lib+bins+examples, `-D warnings`
  plus the listed `-D` lints)
- `cargo test -p titania-lanes` (touched crate, fully green) plus
  `cargo test -p titania-check --test cli_dispatch` if the CLI regression is added
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features --no-deps`

Excluded lanes (NOT evidence, recorded as residuals): `moon ci`,
Kani harnesses, cargo-mutants, Miri/loom/fuzz.

Global-state impact: none — change is additive (one new module, one new
error variant, tests). No existing behavior altered except that previously
unchecked `extra_args` are now rejected (all in-repo callers pass `&[]`,
so no caller migration needed).
