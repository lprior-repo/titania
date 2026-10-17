# tn-ebn9 Test-Plan Review

reviewer_skill: test-reviewer
reviewer_invocation_id: s10.tn-ebn9.test-reviewer
writer_invocation_id: s8.tn-ebn9.test-planner

STATUS: APPROVED

Behavior-test review only (no proof re-review):

- Every planned obligation has both a behavior test and a disjoint
  refinement harness; matrices in `test-plan.md`/`test-writer-report.md`
  agree cell by cell.
- Assertions are exact (rule ids, effects, repair classes, exit codes,
  span coordinates, digest shapes) — no `is_ok`-only or count-only
  assertions on behavior paths.
- Deterministic: fixture sources pinned in-repo; temp workspaces copied,
  never mutated in place; no wall-clock, network, or RNG dependence in
  assertions (the two template_prepush network failures are out of scope
  and recorded as global blockers, not behavior evidence).
- Guards against the reported failure mode exist as negative tests with
  named-rule assertions.
