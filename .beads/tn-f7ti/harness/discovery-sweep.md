# Discovery-sweep procedure — refinement harness (SD)

1. Add a new file verification/verus/scratch_note.vr containing a genuine
   proof-shaped body; run the live inventory; expect
   UnregisteredProductionProof naming the new path.
2. Replace its body with a comment-only note; rerun; expect the inventory
   to stay clean (nothing to dispose).
3. Remove the scratch file; rerun; expect clean.
4. Confirm the smoke fixture row still carries the smoke-fixture authority
   in contracts/proof_obligations.yaml; flip it to production locally and
   confirm the inventory fails with RegisteredSmokeAsProduction; revert.

Steps 1–3 mirror unregistered_production_proof_requires_disposition;
step 4 mirrors smoke_fixture_cannot_satisfy_a_production_obligation.
