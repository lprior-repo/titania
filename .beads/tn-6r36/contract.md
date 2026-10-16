# tn-6r36 Contract — Phase 1 Enforcement Policy (Summary)

Bead: `tn-6r36` (P0, in_progress). This file is the contract deliverable
index for proof-planner, test, and implementation lanes. Contract only —
no source changes land from this bead.

## Status

- Research read: `v1-spec.md` (enforcement chapters), `README.md`,
  `clippy.toml` (strict bans), `crates/titania-policy/src/lib.rs` +
  `exceptions.rs` (approval authority shape),
  `crates/titania-lanes/src/policy_scan/cargo_lints.rs` (weakening floor),
  `crates/titania-dylint/src/functional_surface.rs` + `lib.rs` (current
  syntax-level enforcement baseline).
- Current baseline: `functional_surface.rs` rejects all `for`/`while`/`loop`
  and all `unwrap_or*` by name; `method_policy` matches spelling only and
  skips `from_expansion` spans. This contract redefines the target
  classifier without editing that code.

## Decisions

| # | Decision | Choice |
|---|----------|--------|
| D1 | Boundedness criterion | Named bound + cited evidence; syntax irrelevant (INV-1). |
| D2 | Recovery criterion | Typed-failure consumption + live approval; else erasure (INV-2). |
| D3 | Approval authority | `exceptions.toml` entry (owner, reason, `YYYY-MM-DD` expiry, review); expired = absent; no auto-approval. |
| D4 | Critical designation | Maintained contracts only (`v1-spec.md`, strict-ai profile, `.titania/profiles/`). |
| D5 | Unproven obligations | Block critical shipping; `SKIP`/timeout/`UNPROVEN`/missing tool never counts (INV-3). |
| D6 | Proof standing | Listings, coverage %, `cover!` are non-vacuity only, never execution proof (INV-5). |
| D7 | CI | Moon only; no GitHub Actions restoration (INV-4). |
| D8 | Landing | No relaxation of current enforcement from this bead. |

## Artifacts

| Artifact | Path |
|----------|------|
| Domain model | `domain-model.md` |
| Type contracts | `type-contracts.md` |
| Workflow model | `workflow-model.md` |
| Error taxonomy | `error-taxonomy.md` |
| Boundary map | `boundary-map.md` |
| Hazard analysis | `hazard-analysis.md` |
| Proof seeds | `proof-seeds.jsonl` |
| Traceability | `traceability-matrix.jsonl` |

## Ubiquitous language (excerpt)

Bounded execution, loop syntax, named bound, termination evidence, typed
failure, contract-approved recovery, error erasure, recovery approval
authority, critical module, required obligation. See `domain-model.md`.

## Open domain questions

None. D1–D8 locked. Follow-ups own implementation: `tn-ebn9`
(bounded-execution checks), `tn-8obr` (approved recovery), `tn-pfp1`
(resolved method identities), `tn-ce7x` (obligation gate).

## Illegal states that remain representable

- A `TerminationClaim` can cite a test that does not exercise the bound;
  reviewers must check the evidence ref (H4).
- An approval can expire between classification and shipping; evaluation
  date must be ship-time, not author-time (H2).

## Proof seeds emitted

9 seeds: 6 proptest, 1 Kani, 1 Verus, 1 fuzz. See `proof-seeds.jsonl`.

## Next steps

proof-planner owns lane decisions; proof-plan-reviewer must accept every
lane before proof-writer. Then `tn-ebn9`/`tn-8obr`/`tn-pfp1` implement the
classifier; `tn-ce7x` wires the critical gate.
