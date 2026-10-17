# Machine Gate Report — tn-bvv7 (formal-verifier)

reviewer_skill: formal-verifier
reviewer_invocation_id: s12.tn-bvv7.formal-verifier

Executed in the isolated worktree (`TMPDIR=/dev/shm/tn-bvv7-tmp`;
`RUSTC_WRAPPER=""` where sccache would touch over-quota `/tmp`):

- `cargo fmt --all -- --check` → exit 0.
- `cargo check --workspace --all-targets --all-features` → exit 0.
- Source-only strict clippy (AGENTS.md flag set) → exit 0.
- `cargo test -p titania-lanes -p titania-core -p titania-dylint` →
  848 passed, 0 failed.
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features
  --no-deps` → exit 0.
- Proptest obligations: 2 × 64 cases → exit 0.

Raw logs: `evidence/gate-fmt.txt`, `evidence/gate-check.txt`,
`evidence/gate-clippy.txt`, `evidence/touched-crates-test.txt`,
`evidence/gate-doc.txt`, `evidence/proptest-obligations.txt`,
`evidence/failing-first-detectors.txt` (pre-fix failure).

Not run per parent direction: full `moon ci`, kani, mutants
(owned by tn-4isu/tn-7bq2). No global pass claimed.
