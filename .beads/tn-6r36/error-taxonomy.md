# tn-6r36 Error Taxonomy (`thiserror`, railway)

```rust
pub enum ContractError {
    #[error("empty bound cap")] EmptyBound,
    #[error("missing termination claim")] MissingBound,
    #[error("unbounded retry despite combinator syntax")] UnboundedDespiteCombinators,
    #[error("error erasure without live approval")] ErrorErasure,
    #[error("recovery approval expired on {today}")] ApprovalExpired { today: PolicyDate },
    #[error("approval field missing: {field}")] MissingApprovalField { field: &'static str },
    #[error("unproven required obligation blocks critical shipping")] CriticalGateBlocked,
    #[error("required evidence absent or contradictory")] EvidenceAbsent,
}
```

Rules: large/unit-error lints respected (`ContractError` payloads are typed,
no `String` arms); every fallible boundary returns `Result<_, ContractError>`;
shell maps to `anyhow` with context only at the process edge. `ErrorErasure`
covers `unwrap_or*`, `let _ =`, and bare-statement discards uniformly —
early guard drop of a non-`Result` binding is not `Result` handling and must
not be conflated (tn-8qfc boundary).
