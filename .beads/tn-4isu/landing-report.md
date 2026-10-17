# Landing report — tn-4isu (State 15)

## Integration: NOT PERFORMED (no authorization)

- No commit, no push, no Dolt/remote sync — explicitly unauthorized by the
  delegating task and the conservative profile. Worktree left in place.
- `git status`: 1 modified file (`crates/titania-core/src/kani.rs`, 13+/3-,
  attributes + comment only) + untracked bead artifacts under
  `.beads/tn-4isu/` (evidence, transcripts, reports — handoff material, not commits).
- Bead NOT closed: acceptance unmet (3 structural blockers stand); `bd` update
  with evidence summary is the parent's call on receipt of this report.

## Remote reachability: NOT APPLICABLE

No remote operation attempted or authorized. Nothing to verify.

## Bead close/sync: DEFERRED TO PARENT

- Suggested parent action: `bd update tn-4isu` with the evidence summary from
  final-evidence-decision.md (9/12 PASS + 3 numbered blockers); keep OPEN.
- Follow-up seeds for whoever continues: (a) solver-side options for the three
  blocked harnesses (beyond sound harness scoping — e.g., upstream CBMC/serde
  model improvements); (b) re-run the 3 heavies on faster hardware with the
  same caps to refresh numbers; (c) package-split ONLY if per-heavy <600s
  completions are ever established.

## Handoff contents

Worktree: `/home/lewis/src/titania/.muse/worktrees/subagent-v2-01a14623-5a8f-76e0-be4a-be4eb153aaf7-01a146dc-32cc-7d23-8f2f-dcaec7895c38`
Source diff: `git diff` (1 file). Evidence: `.beads/tn-4isu/evidence/` (16 kani
logs + 5 gate logs). Reports/ledgers: `.beads/tn-4isu/` (states 1–14 PASS).
