# tn-pdn — Error Taxonomy

## Error Hierarchy

```
Error (sealed trait)
├── ReportError          ── report assembly invariants
│   ├── EmptyReject      ── Reject with both collections empty
│   └── EmptyPerLane     ── Pass with empty per_lane
│
├── RuleIdError          ── rule identifier validation
│   ├── Empty            ── zero-length string
│   ├── NoUnderscore     ── no underscore found
│   └── NotUppercase     ── lowercase/non-ASCII character
│
├── LaneError            ── lane resolution
│   └── UnknownLane(String) ── unrecognized PascalCase lane name
│
├── GateScopeError       ── scope resolution
│   └── UnknownScope(String) ── unrecognized scope string
│
├── OutcomeError         ── lane outcome validation
│   ├── Argv0Mismatch { expected, found } ── command binary mismatch
│   └── (others)
│
├── FailureError         ── process termination validation
│   └── InvalidSignal(i32) ── signal outside 1-31
│
├── ReceiptError         ── receipt validation
│   └── SchemaVersionMismatch ── schema_version != 1
│
├── RepairHintError      ── repair hint validation
│   └── EmptyRange ── Patch with zero-width range
│
├── LocationError        ── location validation
│   └── (span validation errors)
│
├── WorkspacePathError   ── workspace path validation
│   └── (path normalization errors)
│
├── PolicyDiagnostic     ── policy configuration error
│   └─ message, file?, severity
│
└── InputDiagnostic      ── CLI input error
    └─ message, tool?, severity
```

## Railway of Error Flow

### Happy Path (Pass)

```
Good input → Policy loads → Lanes execute (all Clean/Skipped) → Pass { receipt } → Exit 0
```

### Code Finding Path (Reject — CodeOnly)

```
Bad code → Policy loads → Some lanes produce Finding { effect: Reject } →
  assemble_report → Reject { code_findings, gate_failures: [], per_lane } →
  RejectKind::CodeOnly → Exit 1
```

### Infrastructure Failure Path (Reject — GateOnly)

```
Good code → Policy loads → Lane fails (e.g., Dylint binary missing) →
  assemble_report → Reject { code_findings: [], gate_failures, per_lane } →
  RejectKind::GateOnly → Exit 1
```

### Mixed Path (Reject — Mixed)

```
Bad code + infra failure → Policy loads → Findings + LaneFailure →
  assemble_report → Reject { code_findings, gate_failures, per_lane } →
  RejectKind::Mixed → Exit 1
```

### Policy Error Path

```
Bad policy → Policy parse fails → PolicyError { diagnostics } → Exit 1
```

### Input Error Path

```
Bad CLI args → Input validation fails → InputError { diagnostics } → Exit 1
```

## Error Categories by Origin

### Domain Errors (invariants on domain values)
- `ReportError::EmptyReject` — violated by `assemble_report` invariant check
- `ReportError::EmptyPerLane` — violated by Pass invariant check
- `RuleIdError::*` — violated by `RuleId::new` constructor
- `RepairHintError::EmptyRange` — violated by `RepairHint` smart constructor

### Lane Errors (tool execution)
- `LaneFailure::Infra { tool, reason }` — tool not found or binary missing
- `LaneFailure::ToolFailure { tool, ProcessTermination }` — tool exited non-zero
- `LaneFailure::SuspiciousFailure { tool, evidence }` — tool produced unexpected output
- `LaneFailure::ResourceFailure { tool, limit }` — timeout/memory exceeded

### Parsing Errors (wire deserialization)
- `LaneError::UnknownLane` — unknown PascalCase lane in JSON
- `GateScopeError::UnknownScope` — unknown scope string
- `ReceiptError::SchemaVersionMismatch` — receipt schema_version != 1
- `Report` deserialization rejects unknown fields via `deny_unknown_fields`

## Killer Demo Error Mapping

| Scenario | Expected Error | Source |
|----------|---------------|--------|
| Bad fixture rejected | `Report::Reject { CodeOnly }` | `assemble_report` |
| Bad fixture missing Cargo.toml | `InputError { diagnostics }` | CLI input validation |
| Repaired fixture passes | `Report::Pass { schema_version=1 }` | `assemble_report` |
| Dylint binary missing in test | `LaneFailure::Infra` in gate_failures | Dylint lane runner |
| Invalid scope string | `InputError { "unrecognized scope" }` | CLI parser |
| Policy file missing | `PolicyError { "policy file missing" }` | Policy loader |

## Finding-to-Error Mapping

A `Finding` with `effect: Reject` does NOT produce an error — it's a code finding that goes into `code_findings`. The report itself (`Reject`) is not an error type; it's a domain value. Errors (`ReportError`, `LaneError`, etc.) only arise from invariant violations or infrastructure failures.
