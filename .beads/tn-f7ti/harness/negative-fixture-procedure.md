# Negative-fixture procedure — refinement harness (RC)

Run each paired control against the audit entry point and confirm the
named consumer-visible reason:

1. Delete the receipt file from the in-memory store; expect
   RegisteredMissing naming verification/verus/receipt_schema.rs.
2. Replace receipt content with `fn main() {}`; expect RegisteredProofFree
   naming the same obligation.
3. Drop the receipt row from the registry; expect
   RequiredObligationRemoved naming the same obligation.
4. Register the smoke fixture as production; expect
   RegisteredSmokeAsProduction naming
   verification/verus/formal_setup_smoke.rs.
5. Feed `// proof fn ensures` plus a quoted "assume_specification" line;
   expect ProofFree (comments and strings prove nothing).

All five are automated in crates/titania-lanes/tests/proof_registry.rs;
this procedure is the human-runnable counterpart used at review time.
