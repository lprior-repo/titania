# tn-6r36 Hazard Analysis

- H1 Temporal: unbounded retry relabeled as combinator chain ships as
  "bounded". Mitigation: INV-1 (syntax never proves bound); transition 3
  rejects unclaimed sites; regression test with combinator-spelled retry.
- H2 Invariant: `unwrap_or_else` with live approval drifts into blanket
  default once approval expires. Mitigation: expiry enforced per evaluation
  date; expired equals absent; `ApprovalExpired` typed error.
- H3 Bounded state: `Bound::Capped { max: 0 }` or overflow cap accepted.
  Mitigation: smart constructor rejects zero; caps are `u64` with evidence
  citing the constant and its test.
- H4 Refinement: `TerminationClaim` cites a test that does not exercise the
  bound. Mitigation: evidence ref must name a regression test asserting the
  bound (input-length identity or cap exhaustion path).
- H5 Concurrency: n/a — classifier is pure sync; no threads, locks, or
  tasks in this bead. If execution parallelizes later, Loom owns the lane.
- H6 Unsafe/provenance: none in contract scope.
- H7 Hostile input: malformed `exceptions.toml` (empty owner, bad date,
  unknown fields) treated as approval. Mitigation: strict parse
  (`deny_unknown_fields` parity, non-empty checks, `YYYY-MM-DD` validation);
  malformed file yields `MissingApprovalField`/`ApprovalExpired`, fail closed.
- H8 Performance/release: classification must stay a cheap pure function;
  no proof-tool or I/O execution in the hot path (compile complexity to IR,
  runtime applies bounded transitions).
- H9 Evidence integrity: green build cited as enforcement proof.
  Mitigation: INV-5; `EvidenceAbsent` rejects listings, percentages, and
  `cover!` non-vacuity as execution proof; release needs raw command logs.
