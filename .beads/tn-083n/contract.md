# Contract — tn-083n trust/binding audit

## Ubiquitous language

- **Source linkage**: a `#[path]` import plus `assume_specification`
  bridge. Names a production symbol; proves nothing about its body.
- **Production behavior verification**: a passing verifier run over the real
  executable body, recorded as raw evidence (tool, version, input revision,
  exit status) whose revision matches the current production hash.
- **Mirror**: a verification file that restates production behavior
  (`assume_specification` / copied model). Drift = production hash moved on
  while the mirror/evidence did not.
- **Trust marker**: `assume_specification`, `assume(`, `external_body`,
  `admit`, `trusted`. Allowed only with an independent explicit waiver, and
  a waiver never promotes linkage to proof.
- **Smoke fixture**: a file marked `fixture-smoke` (or smoke-named). Never
  satisfies a production obligation.
- **Vacuous proof**: proof items that never reference the bound production
  symbol (e.g. a `2 == 2` theorem). Cannot discharge schema behavior.

## Value objects

- `Obligation { id, production_path, production_symbol, proof_file }`
- `VerifierEvidence { obligation_id, tool, tool_version, revision, exit_code }`
- `Waiver { obligation_id, marker, approver, reason }`
- `FileSignals` (pure scan result: path refs, marker hits with lines,
  production-ref count, proof-fn count, smoke flag)
- `BindingVerdict`: `VerifiedProduction { binding, trusted_base }` |
  `SourceLinked { waived_markers }` | `Blocked { reason }`
- `BlockReason`: `SmokeOnly | UnapprovedTrust | MissingBinding |
  MirrorDrift | VacuousProof | MissingEvidence`

## Decision table (audit order)

1. Smoke fixture → `Blocked(SmokeOnly)`.
2. Trust marker without covering waiver → `Blocked(UnapprovedTrust)`.
3. No `#[path]` and zero `production::` refs → `Blocked(MissingBinding)`.
4. Zero `production::` refs despite proof items → `Blocked(VacuousProof)`.
5. No passing evidence at current revision → `SourceLinked` (no evidence)
   or `Blocked(MissingEvidence)` when exit != 0 … see below.
6. Evidence revision != current production hash → `Blocked(MirrorDrift)`.
7. Passing evidence at current hash → `VerifiedProduction`.

Step 5 refined: bound-but-unverified with clean markers classifies
`SourceLinked` (informational, no finding); evidence with nonzero exit
classifies `Blocked(MissingEvidence)` (finding). Unbound obligation with
no proof file at all → `Blocked(MissingBinding)`.

## Findings (lane rule ids)

- `TRUST_UNAPPROVED_MARKER`, `TRUST_MISSING_BINDING`,
  `TRUST_MIRROR_DRIFT`, `TRUST_SMOKE_COVERAGE`, `TRUST_VACUOUS_PROOF`,
  `TRUST_MISSING_EVIDENCE` — one per blocked obligation, file-level
  (`line = 0`) on the proof file, or on the obligation id when the proof
  file is absent.

## Invariants

- No linkage, trust declaration, listing, or coverage value is ever
  reported as production-execution proof.
- Waivers suppress the marker finding only; classification stays
  `SourceLinked`.
- All inputs are caller-supplied typed values; the audit performs no I/O.
