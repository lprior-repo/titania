reviewer_skill: black-hat-reviewer
reviewer_invocation_id: tn-c6n-inv-013
writer_invocation_id: tn-c6n-inv-012

# Black-Hat Review — tn-c6n (State 13)

STATUS: APPROVED

## Scope attacked

- Contract: `.beads/tn-c6n/contract.md` (R1–R6) through
  `.beads/tn-c6n/verification-ledger.jsonl` (14 rows, all PASS).
- As-built tests: `crates/titania-core/tests/receipt_domain_invariants.rs`
  (14 checks), `crates/titania-core/tests/receipt_domain_properties.rs`
  (5 proptest equivalences + 1 stability check).
- Production: `crates/titania-core/src/receipt.rs`,
  `crates/titania-core/src/receipt/{lane_name,schema,target_root,serde_support,digests}.rs`,
  `crates/titania-core/src/error.rs::ReceiptError`,
  `crates/titania-core/src/v1_receipt.rs::QualityReceipt::new`.
- Evidence: `.beads/tn-c6n/evidence/{cargo-test-invariants-s12,cargo-test-properties-s12,verus-s12,red-control,check-s12,clippy-s12,fmt-s12}.log`.

Method: read every artifact above in full; cross-checked each ledger
`raw_log` claim against its evidence file; grepped for laundering markers
(`external_body`, `#[trusted]`, `assume_specification`, `scratch_`,
`cover!`); verified `git status --porcelain -- crates/ verification/`
shows only the two new test files (zero production drift).

## Phase 1 — Contract & bead parity: PASS

Every R1–R6 clause has executed evidence calling production constructors
directly; no clause rests on design-model reasoning. Ledger-to-log counts
match exactly (14 passed / 6 passed / `3 verified, 0 errors`).

## Phase 2 — Farley rigor: PASS

Production functions are all under the line ceiling with pure
check-then-construct shape; no I/O inside calculations (receipt core is
filesystem-free; `TargetProject` I/O lives outside the receipt boundary).
The one long function (`receipt_error_match_is_exhaustive_with_stable_messages`,
~70 lines) is a test; per this skill's Phase 2, test implementation style
is out of scope absent weakened assertions — its assertions are exact
(variant + full message string), so it stands.

## Phase 3 — Holzman Big 6: PASS

Illegal states unrepresentable (newtypes + smart constructors on every
receipt type); parse-don't-validate holds (`LaneName`, `LaneDigest`,
`RecordedTargetRoot`, `ReceiptEnvelope` Deserialize all funnel through
`new`); no boolean parameters; no stringly domain IDs; no unwrap/expect/
panic in touched production (tests-only delivery, zero prod files changed).

## Phase 4 — Simplicity & DDD: PASS

No Option-based state machines; no speculative generality (private
`from_parts` and `pub(super)` schema predicate both have live callers);
test `unwrap`s are guarded by same-statement accept/reject predicates or
hardcoded-valid fixtures — none weakens an assertion.

## Phase 5 — Bitter truth: PASS

Boring, obvious code. The delivery's own documents confess the weak points
up front (vacuous `2 == 2` lemma, assumed binding contract, kani absent);
there is nothing clever left to punish.

## Laundering hunt: clean

- Exactly one `assume_specification`
  (`verification/verus/receipt_schema.rs:8`), ledgered as TBL `tn-c6n.T1`
  (accepted) with real compensating evidence `O-P6`. No `external_body`,
  no `#[trusted]`, no hardcoded-input Kani games, no `cover!`-as-proof.
- The vacuous lemma is labeled vacuous with zero behavior weight in three
  separate artifacts. Weight-laundering was attempted by nobody.
- Red control is real: `evidence/red-control.log` shows a scratch test
  asserting broken behavior (`scratch_red_control_empty_name_is_accepted`)
  FAILING against correct production while the other 14 passed — the suite
  bites. `grep scratch_ crates/` returns empty: residue removed.

## Proof/Test/Source Parity Matrix

| Req | Proof evidence | Test evidence | Source ref | Attack mounted | Result |
|---|---|---|---|---|---|
| R1 lane-name | O-P1/B-P1 proptest equivalence, 256 cases incl. empty+NUL, PASS (`cargo-test-properties-s12.log`, 6 passed) | 3 behavior tests: empty→`EmptyLaneName`, NUL→`InvalidLaneName`+JSON reject, round-trip preserve | `receipt/lane_name.rs::LaneName::new` | lossy-UTF8 corpus masking? long-name bound? — production takes `String`, no length bound; lossy output is exactly what production sees | HOLD |
| R2 lane-digest | O-P2/B-P2 full u32×u32 equivalence with exact `{passed, scanned}` payload assert, PASS | constructor reject with exact message + `(3,3)`/`(0,0)` boundaries + JSON reject | `receipt.rs::LaneDigest::new` | `finding_count` unconstrained — production genuinely ignores it; no gap | HOLD |
| R3 error-exhaust | O-P3/B-P3 closed 9-variant enumeration, PASS | exhaustive `match` (no wildcard: 10th variant breaks build) + `Display == expected` per variant; `error_messages_stable` distinctness | `error.rs::ReceiptError` + `v1_receipt.rs::QualityReceipt::new` (EmptyLaneReceiptList path, line 127) | hand-mirrored `rendered` strings could drift together? — `Display` is compared against independent literals, so prod message change fails the test | HOLD |
| R4 period | O-P4/B-P4 full u64×u64 equivalence incl. equal stamps and `(MAX,0)`, exact payload assert, PASS | equal-accept + reversed-reject with exact variant | `receipt.rs::ReceiptPeriod::new` | off-by-one on equality? — both property (`finished >= started`) and behavior test pin equal-accept | HOLD |
| R5 target-root | O-P5/B-P5 string-corpus equivalence `!empty && absolute && !NUL`, PASS | empty/relative/NUL rejections with exact variants + `manifest_path` join | `receipt/target_root.rs::RecordedTargetRoot::new` | `starts_with('/')` vs `Utf8Path::is_absolute` divergence? — identical on Unix/camino; precedence of relative-before-NUL unpinned but contract does not specify precedence | HOLD |
| R6 schema-gate | O-P6/B-P6 arbitrary-u32 property + sampled `0,1,3,MAX`, exact message, PASS; O-V6/B-V6 verus binding `3 verified, 0 errors` claimed as hygiene only | v2 accept + non-v2 reject loop over `[0,1,3,MAX]` with message assert | `receipt/schema.rs::RECEIPT_ENVELOPE_SCHEMA_VERSION` (= 2); `serde_support.rs` checks schema before body (fail-closed) | verus overclaim? — artifacts explicitly assign it zero behavior weight; Deserialize bypass of `from_parts`? — schema is checked twice (pre-wire + `from_parts`) | HOLD |
| Forgery check | ledger `raw_log` counts vs evidence files | 14/14, 6/6, verus tail all byte-match their logs | n/a | stale `traceability-matrix.jsonl` (rows still `contracted`, null IDs) — superseded by alignment artifacts; cosmetic, non-behavior | HOLD (observation O1) |

## Observations (non-blocking, repair optional)

- O1: `traceability-matrix.jsonl` was never filled forward (all rows still
  `status: contracted`, null obligation/ledger IDs) despite contract.md
  promising forward-fill. Superseded in practice by
  `proof-test-source-alignment.jsonl` + `verification-ledger.jsonl`.
  Suggested repair: update the six rows or repoint contract.md at the
  alignment artifacts.
- O2: `receipt.rs:169` doc comment says "not the current v1 receipt schema"
  where it means the v2 envelope namespace; `check_supported_schema_version`
  doc has the same stale "v1". Cosmetic; unpinned by any test.
- O3: ledger row `tn-c6n.L-O-V6` sets `behavior_affecting: true` while the
  narrative claims zero behavior weight. This errs conservative (demands
  evidence rather than excusing it) — not laundering — but the flag and the
  prose disagree; align one of them.

## Verdict

No parity gap, no theater, no weak test, no drift, no forgery found after
good-faith attack on all five phases. The red control proves the suite
fails broken production; the exhaustive match proves R3 at compile time;
the full-domain properties prove R1/R2/R4/R5/R6 at runtime; the verus run
is honestly scoped to binding hygiene.

STATUS: APPROVED
