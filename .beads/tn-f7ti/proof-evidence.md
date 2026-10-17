# Proof evidence — tn-f7ti

writer_skill: proof-writer
writer_invocation_id: INV-06-proof-writer

STATUS: APPROVED

Behavior checks executed (evidence/proof_registry-tests.log, exit 0):

- Happy: registered proofs accounted by stable identity; genuine `.vr`
  content counts by content.
- Error: smoke-as-production fails named; deleted file fails named;
  proof-free replacement fails named; removed required row blocks;
  unregistered production proof requires disposition.
- Edge: comment/string mentions are not proof; extension alone is not
  proof; smoke tag wins over tokens.
- Boundary: checked-in registry parses with smoke authority; unknown
  fields fail closed; live workspace inventory is clean.

The suite ran before implementation (red: missing module) and after
(green: 13 of 13). Raw log and exit status are retained under evidence/.
All claims above resolve to lines in that log.
