# Type contracts — tn-c6n

## `LaneName` (`crates/titania-core/src/receipt/lane_name.rs`)

- `new(impl Into<String>) -> Result<Self, ReceiptError>`: `""` →
  `Err(EmptyLaneName)`; any NUL byte → `Err(InvalidLaneName)`; else the exact
  input is stored.
- `as_str() -> &str`, `Display`, `Serialize` preserve the stored text exactly.
- `Deserialize` funnels through `new`: JSON text cannot bypass validation.

## `LaneDigest` (`crates/titania-core/src/receipt.rs`)

- `new(lane, exit, scanned, passed, finding_count)`: `passed > scanned` →
  `Err(PassedExceedsScanned { passed, scanned })`; boundary `passed ==
  scanned` and `(0, 0)` are accepted.
- Accessors `lane/exit/scanned/passed/finding_count` are total.
- `Deserialize` funnels through `new` via the wire struct.

## `ReceiptPeriod` (`crates/titania-core/src/receipt.rs`)

- `new(started_at, finished_at)`: `finished_at < started_at` →
  `Err(FinishedBeforeStarted { .. })`; equal stamps accepted.

## `RecordedTargetRoot` (`crates/titania-core/src/receipt/target_root.rs`)

- `new(impl Into<Utf8PathBuf>)`: empty → `TargetRootEmpty`; relative →
  `TargetRootNonAbsolute(path)`; NUL → `TargetRootContainsNul`.
- `manifest_path()` appends `Cargo.toml` to the recorded root.

## `ReceiptEnvelope` (`crates/titania-core/src/receipt.rs`, `schema.rs`, `serde_support.rs`)

- `RECEIPT_ENVELOPE_SCHEMA_VERSION: u32 = 2`.
- `new(...)` always stamps schema 2. `from_parts` rejects any other version
  with `UnsupportedSchemaVersion(v)`. `Deserialize` checks the schema before
  any other field, so stale shapes fail closed on version, not on shape.

## `ReceiptError` (`crates/titania-core/src/error.rs`)

Nine variants, each with a fixed `thiserror` message; every variant is
produced by at least one production path above (plus the v1
`EmptyLaneReceiptList` path).
