# Baseline report — tn-c6n (2026-10-16, worktree at 927893f)

## Identification

- `pwd`: worktree above. `git status --short`: clean. `git log -1`: 927893f.
- Toolchain file: `rust-toolchain.toml` → nightly-2026-04-27.

## Targeted baseline (raw)

- `cargo fmt --all -- --check` → exit 0, no diff.
- `cargo test -p titania-core --test receipt_invariants` → 9 passed, 0 failed.
- `verus --version` → `Verus 0.2026.05.05.d03e906, Profile: release`.
- `verus verification/verus/receipt_schema.rs` → `verification results:: 3 verified,
  0 errors` in 0.393s.
- `command -v kani` → exit 1 (absent). `command -v flux` → present.
- Registry `contracts/proof_obligations.yaml` lists only `formal_setup_smoke.rs`
  and `receipt_schema.rs`; no `receipt_invariants.rs` is registered or present.

## Production ground truth (read, not changed)

- `crates/titania-core/src/receipt/lane_name.rs`: `LaneName::new` rejects empty
  (`EmptyLaneName`) and NUL (`InvalidLaneName`); serde path reuses `new`.
- `crates/titania-core/src/receipt.rs`: `LaneDigest::new` rejects
  `passed > scanned`; serde path reuses `new`; `ReceiptPeriod::new` rejects
  `finished_at < started_at`; envelope `from_parts` gates schema version.
- `crates/titania-core/src/receipt/target_root.rs`: `RecordedTargetRoot::new`
  rejects empty / relative / NUL paths.
- `crates/titania-core/src/receipt/schema.rs`: `RECEIPT_ENVELOPE_SCHEMA_VERSION
  = 2`; `is_supported_receipt_schema_version` is `pub(super) const fn`.
- `crates/titania-core/src/error.rs`: `ReceiptError` has 9 variants, all with
  stable `thiserror` messages.

## Gaps (what this bead adds)

No behavior tests call `LaneName::new` rejection paths, `RecordedTargetRoot`
rejection paths, boundary acceptances (`passed == scanned`, `0/0`, equal
timestamps), `ReceiptLaneExit` round-trip, or exhaustive `ReceiptError`
matching. No proptest properties exist for the receipt domain. No pinned
`verus` raw output is recorded in bead evidence.
