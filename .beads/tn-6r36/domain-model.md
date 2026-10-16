# tn-6r36 Domain Model — Phase 1 Enforcement Contract

Bead: `tn-6r36` (P0, in_progress). Scope: contract + traceability only. No source changes.

## Ubiquitous language

- **Bounded execution** — A computation with a named, auditable termination
  argument: a finite input collection, a numeric iteration cap with evidence,
  or a protocol state machine with an explicit terminal state.
- **Loop syntax** — Surface form: `for`, `while`, `loop`, or an iterator
  combinator chain (`map`/`filter`/`fold`/`try_fold`/`try_for_each`). Syntax
  alone never proves boundedness.
- **Named bound** — The declared limit: `Bound::FiniteInput`, `Bound::Capped(n,
  evidence)`, or `Bound::ProtocolTerminal`. Must cite evidence (input length,
  cap constant + test, state-transition table).
- **Termination evidence** — The artifact proving the bound holds: input-length
  identity, cap constant with regression test, or transition-table coverage.
- **Typed failure** — A `thiserror` domain error or `Result<T, E>` payload
  carrying cause and context. Never a stringly default.
- **Contract-approved recovery** — Handling that consumes a typed failure
  explicitly (`match`, `map_or_else`, typed default type) under a live
  `exceptions.toml` approval (owner + reason + expiry + review).
- **Error erasure** — Hiding a failure with a default (`unwrap_or*`,
  `let _ =`, bare statement) without consuming the typed cause and without a
  live approval. Always rejected.
- **Recovery approval authority** — The `exceptions.toml` entry for a
  `(rule_id, path)` pair: owner, reason, ISO-8601 expiry, review ticket.
  Expiry is enforced; expired approval equals no approval. No auto-approval.
- **Critical module** — A module designated by maintained contracts
  (`v1-spec.md`, `titania-policy` strict-ai profile, `.titania/profiles/`).
  Unproven required obligations block shipping for critical modules.
- **Required obligation** — A proof/test gate the contract marks mandatory
  (behavior-affecting lanes). `SKIP`, timeout, `UNPROVEN`, absent tool, or
  contradictory evidence never counts as satisfaction.

## Entities / value objects / aggregates

- `Bound`, `TerminationClaim`, `RecoveryApproval`, `CriticalDesignation`,
  `ObligationState` (see `type-contracts.md`).
- Aggregate root: `EnforcementClassification` — one judgment per code site:
  input facts (syntax form, bound, failure handling, approvals) → verdict
  (`Permitted` | `Rejected`) + reason code.

## Commands / events / policies

- Command: `ClassifySite(site_facts)` → Event: `SitePermitted` | `SiteRejected`.
- Policy INV-1: syntax never implies boundedness.
- Policy INV-2: recovery without consumed cause + live approval is erasure.
- Policy INV-3: unproven required obligation blocks critical shipping.
- Policy INV-4: Moon is the only CI/CD orchestrator.
- Policy INV-5: no linkage/listing/coverage/`cover!` counts as execution proof.
