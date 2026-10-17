# Trusted-Base Plan — tn-bvv7

No proof markers are used anywhere in this bead: no `assume`-style
hints, no axioms, no admitted lemmas, no sorry placeholders, no
`external_body` bodies, no `trusted`/`ignore`/`stub` annotations, no
disabled checks, no model bounds or reductions.

Plan: scan `proof-evidence.md`, `proof-writer-report.md`, and
`proof-review.md` for the marker set; record every hit in
`trusted-base-ledger.jsonl`, otherwise record one closing row attesting
a clean scan with the exact grep evidence. All rows carry
`reviewer_disposition: accepted` and non-pending status before closure.
Behavior-affecting markers would block; none exist.
