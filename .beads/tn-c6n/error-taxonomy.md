# Error taxonomy — tn-c6n (`ReceiptError`, 9 variants)

| Variant | Producer | Message |
|---|---|---|
| `UnsupportedSchemaVersion(u32)` | `from_parts`, envelope `Deserialize` | `unsupported receipt schema version {0}` |
| `EmptyLaneName` | `LaneName::new`, lane `Deserialize` | `lane name must not be empty` |
| `InvalidLaneName` | `LaneName::new`, lane `Deserialize` | `lane name must not contain NUL bytes` |
| `PassedExceedsScanned { passed, scanned }` | `LaneDigest::new`, lane `Deserialize` | `lane passed count {passed} exceeds scanned count {scanned}` |
| `FinishedBeforeStarted { started_at, finished_at }` | `ReceiptPeriod::new` | `receipt finished_at {finished_at} is before started_at {started_at}` |
| `EmptyLaneReceiptList` | v1 `QualityReceipt::new` | `quality receipt must include at least one lane receipt` |
| `TargetRootEmpty` | `RecordedTargetRoot::new` | `receipt target_root must not be empty` |
| `TargetRootNonAbsolute(String)` | `RecordedTargetRoot::new` | `receipt target_root must be absolute, got {0:?}` |
| `TargetRootContainsNul` | `RecordedTargetRoot::new` | `receipt target_root must not contain NUL bytes` |

Railway rule: every fallible receipt API returns `Result<_, ReceiptError>`;
no error is erased, defaulted, or converted to a stringly form inside the
core. Exhaustiveness is enforced by an exhaustive-match behavior test: adding
a tenth variant breaks compilation of the test target.
