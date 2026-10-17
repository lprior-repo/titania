# Hazard analysis — tn-f7ti

H1 — Silent pass on proof-free content. Cause: counting proof words in
comments/strings or trusting extension. Control: token scan runs on
comment- and string-stripped code; extension selects candidates only.
Covered by comment_and_string_mentions_are_not_proof and
extension_alone_is_never_evidence_of_proof.

H2 — Smoke fixture counted as production. Cause: registry rows without
authority. Control: per-entry authority with smoke-fixture marking; smoke
registered as production fails. Covered by
smoke_fixture_cannot_satisfy_a_production_obligation and the checked-in
registry test.

H3 — Required obligation quietly dropped. Cause: registry row removed.
Control: protected required list checked after per-entry verdicts.
Covered by removed_registry_row_hiding_a_required_obligation_blocks.

H4 — Unregistered proof drifts in. Cause: new proof file without registry
row. Control: discovery sweep fails until disposition. Covered by
unregistered_production_proof_requires_disposition.

H5 — Vacuous evidence. Cause: PASS claims without raw logs. Control:
verification ledger rows carry real commands, exit statuses, and log
artifacts; missing tools are blockers, never passes.

Residuals: flat (non-recursive) discovery; lifetime-tick bail may
over-strip pathological ticks (fails loud); identifiers ending in `r`
with a token prefix are theoretical false-hit shapes (none exist in the
token table); no disposition record format is defined yet (failure, not
acceptance, is implemented).
