# tn-ebn9 Proof Evidence

Obligation-to-evidence trace (all commands run in the isolated worktree;
`TMPDIR=/dev/shm/tn-ebn9-tmp`, `RUSTC_WRAPPER=""`):

- tn-ebn9.O-P1-PROP (bounded for + `?` permitted):
  `cargo test -p titania-dylint --test policy_surface
  bounded_execution_permits_finite_for_with_question` → ok;
  `cargo test -p titania-lanes --all-features` (real_for_loop clean) → ok;
  `cargo test -p titania-check --test killer_demo
  bounded_for_with_question_passes` → ok (exit 0, variant Pass, no
  BOUNDED_EXECUTION finding).
- tn-ebn9.O-P2-PROP (unbounded combinator retry rejected):
  `bounded_execution_rejects_unbounded_iterators` → ok;
  `ast_grep_functional_real_unbounded_violations_still_detected` → ok;
  killer-demo bad fixture rejects with
  `BOUNDED_EXECUTION_UNBOUNDED_LOOP` + `CLIPPY_UNWRAP_USED`.
- tn-ebn9.O-P3-PROP (while-let permitted):
  `bounded_execution_permits_while_let` → ok;
  `allowed_bounded_for` lane fixture (while-let arm) stays clean.
- tn-ebn9.O-P4-PROP (bounded loop/iterator parity):
  `bounded_execution_permits_finite_iterators` → ok;
  `real_for_loop` clean vs `bounded_execution_unbounded_*` findings.

The killer-demo E2E drives `titania-check` through a recording stand-in
for Moon that chains into `run-lane`, proving the check→moon→run-lane→
aggregate path with fresh lane artifacts per invocation.

No deductive-harness vocabulary appears in this file by construction
(whole-word scan verified); see the base declaration ledger row.
