# tn-6r36 Type Contracts

## `Bound` (value object, total)

```rust
pub enum Bound {
    FiniteInput,                    // iterates a finite collection by value
    Capped { max: u64, evidence: BoundEvidence }, // numeric cap + cited evidence
    ProtocolTerminal,               // state machine with explicit terminal state
}
```

Smart constructor: `Bound::capped` rejects `max == 0` with
`ContractError::EmptyBound`. No boolean flags; no `Option<u64>` lifecycle.

## `TerminationClaim`

```rust
pub struct TerminationClaim { pub bound: Bound, pub evidence_ref: EvidenceRef }
```

`EvidenceRef` is a newtype over a checked-in path + test name
(`regression: <test fqn>` or `spec: v1-spec.md §<n>`). Parse at boundary;
core accepts only parsed claims.

## `RecoveryApproval`

Mirrors `titania-policy` `Exception`: `(rule_id, path, owner, reason,
expires_on: PolicyDate, review)`. Validation: all fields non-empty,
`expires_on` parses as `YYYY-MM-DD`, `today <= expires_on` else
`ApprovalExpired`. Unknown fields rejected (`deny_unknown_fields` parity).

## `CriticalDesignation`

```rust
pub enum CriticalDesignation { Critical { source: ContractSource }, NonCritical }
pub enum ContractSource { V1Spec, StrictAiProfile, TitaniaProfilesDir }
```

Designation comes only from maintained contracts. No ad-hoc caller flag.

## `ObligationState` (typestate, no `Option`)

```rust
pub enum ObligationState { RequiredUnproven, Proven { evidence: EvidenceRef }, AcceptedWaiver { approval: RecoveryApproval } }
```

There is no `None` state. `RequiredUnproven` on a critical module blocks
shipping by construction (see `workflow-model.md`).

## `EnforcementClassification` (aggregate root)

Inputs are already-parsed domain types: `SyntaxForm`, `Option<TerminationClaim>`
is forbidden — use `TerminationStatus::Claimed(claim) | Unclaimed`.
Output: `Verdict::Permitted | Rejected { code: ReasonCode }`.
All fallible constructors return `Result<_, ContractError>` (`thiserror`);
no `String` errors, no `unwrap`.
