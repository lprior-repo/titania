# Global Readiness Report — tn-bvv7

Targeted gates (this bead, run in the isolated worktree; raw logs under
`evidence/`):

- `cargo fmt --all -- --check`: exit 0 (`evidence/gate-fmt.txt`).
- `cargo check --workspace --all-targets --all-features`: exit 0
  (`evidence/gate-check.txt`).
- Source-only strict clippy per AGENTS.md (`--lib --bins --examples`,
  denied groups plus unwrap/expect/panic/indexing/arithmetic/method bans):
  exit 0 (`evidence/gate-clippy.txt`).
- `cargo test -p titania-lanes -p titania-core -p titania-dylint`:
  848 passed, 0 failed (`evidence/touched-crates-test.txt`).
- `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --all-features
  --no-deps`: exit 0 (`evidence/gate-doc.txt`).

Environment notes (all pre-existing, none caused by this bead):

- `/tmp` is over quota on this machine, so any tool that writes there
  fails. Mitigation used for every gate above: `TMPDIR=/dev/shm/tn-bvv7-tmp`
  (62G free) and `RUSTC_WRAPPER=""` (sccache writes under `/tmp`).
  Proof the quota is pre-existing: `policy_surface` fails identically on
  the untouched base commit 927893f with `Disk quota exceeded`.
- `cargo dylint` subprocess tests scrub `TMPDIR`, so they can only run
  with the wrapper disabled; with `RUSTC_WRAPPER=""` the driver suite
  passes (`dylint_reject_repair`: 1 passed).
- Full `moon ci` and kani/mutant lanes were NOT run per explicit parent
  direction (owned by tn-4isu/tn-7bq2). No global pass is claimed.
