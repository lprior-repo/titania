# Hazard analysis — tn-c6n

1. Mirror drift (HIGH, historical): a copied model of `LaneName`/`LaneDigest`
   can pass while production regresses. Mitigation: no mirror files; every
   test calls production constructors. Residual: none.
2. Source-linkage mistaken for body proof (HIGH, `tn-083n`): the verus schema
   binding is assumed-contract linkage, not constructor verification.
   Mitigation: classified as binding hygiene with zero behavior weight; real
   weight on executed tests. Residual: documented in proof-evidence.
3. Serde bypass (MEDIUM): a hand-written `Deserialize` could skip `new`.
   Mitigation: both impls funnel through `new`; hostile JSON tests prove
   rejection through the text path. Residual: none.
4. Exhaustiveness rot (MEDIUM): a new `ReceiptError` variant could go
   untested. Mitigation: exhaustive-match test fails compilation on new
   variants. Residual: none.
5. Vacuous lemma (LOW): the registered `2 == 2` lemma proves nothing about
   behavior. Mitigation: reported as vacuous; excluded from all behavior
   claims. Residual: file is registry-owned, left untouched.
6. Single-operator review (MEDIUM): no second human reviewer in this run.
   Mitigation: independent workflow-subagent review rounds at States 4b, 6,
   10, 13, 14 with ledgered invocations. Residual: carried to landing.
7. Formal bounded-proof gap (MEDIUM): kani absent + out of scope; flux
   unapplied. Mitigation: proptest rejection/acceptance properties over full
   `u32`/`u64`/string input classes. Residual: kani ownership `tn-7bq2`.
