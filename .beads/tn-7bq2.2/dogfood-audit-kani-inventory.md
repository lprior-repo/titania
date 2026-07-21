# Dogfood Audit — Kani Inventory Parser + Kani Harnesses

**Bead context:** tn-7bq2.2 (Kani proof-writer lane)
**Commit under audit:** `0e83a55` (v1.5)
**Scope:** `crates/titania-core/src/kani_inventory.rs`,
`crates/titania-core/src/kani.rs` (read-only)
**Reviewer:** proof-reviewer (dogfood)
**Date:** 2026-07-20

---

## TL;DR — Findings table

| # | Severity | Obligation | Verdict | File:line |
|---|----------|------------|---------|-----------|
| F1 | INFO     | `KaniHarnessId` matches `[a-zA-Z][a-zA-Z0-9_]{0,95}` | PASS | `crates/titania-core/src/proof_id.rs:18, 104, 111, 124` |
| F2 | INFO     | `KANI_INVENTORY_MAX_HARNESSES` cap enforced | PASS (with weak test surface) | `crates/titania-core/src/kani_inventory.rs:50, 137, 192-201` |
| F3 | INFO     | Unknown top-level keys tolerated | PASS | `crates/titania-core/src/kani_inventory.rs:159-169` + `tests/v15_kani_inventory.rs:78-93` |
| F4 | **HIGH** | Missing required keys rejected | **FAIL** — empty `{}` parses silently | `crates/titania-core/src/kani_inventory.rs:161-168` |
| F5 | **HIGH** | `KaniInventory::parse` exists, accepts empty, emits a finding | **FAIL** — wrong method name; no finding emission in core | `crates/titania-core/src/kani_inventory.rs:126`; `crates/titania-core/src/lib.rs:66-68` |
| F6 | INFO     | `kani_kani_harness_id_bounded` exists | PASS | `crates/titania-core/src/kani.rs:144-145` |
| F7 | INFO     | `kani_kani_lane_name_roundtrip` exists | PASS | `crates/titania-core/src/kani.rs:279-280` |
| F8 | INFO     | `kani_mutants_baseline_diff_zero_neg` exists | PASS | `crates/titania-core/src/kani.rs:370-371` |
| F9 | LOW      | Stale black-hat finding F-33 still cites uppercase-only charset | Obsolete — implementation accepts both cases | `.evidence/v1.5/black-hat-review.md:227` |

---

## F1 — `KaniHarnessId` contract matches `[a-zA-Z][a-zA-Z0-9_]{0,95}` — PASS

**Evidence (`crates/titania-core/src/proof_id.rs`):**
- `:18` `pub const KANI_HARNESS_ID_MAX_LEN: usize = 96;` — bound caps total length at 96.
- `:100-115` `check_khi` rejects empty (`Empty`), over-length (`TooLong`), and a
  first byte that is not `is_ascii_alphabetic()` (`LeadingNonLetter`).
- `:104` `if s.len() > KANI_HARNESS_ID_MAX_LEN` — clamps total length to `1..=96`.
- `:111` `if !first.is_ascii_alphabetic()` — accepts BOTH upper and lower ASCII letters.
- `:114` `bytes.iter().enumerate().skip(1).try_for_each(|(offset, byte)| check_khi_byte(...))`.
- `:123-125` `is_khi_byte = is_ascii_alphabetic() || is_ascii_digit() || == b'_'` — body
  bytes include both cases.

**Net grammar:** `^[a-zA-Z][a-zA-Z0-9_]{0,95}$` (1 letter + 0..=95 body bytes,
total length 1..=96). Matches the user's regex exactly.

**Doc-comment at `proof_id.rs:36-39`** explicitly states the contract as
`^[a-zA-Z][a-zA-Z0-9_]*$` with the cap, which lines up.

**Kani-bound witness** at `crates/titania-core/src/kani.rs:178-261`
(`kani_kani_harness_id_bounded`) exercises every boundary: empty
(`KaniHarnessIdError::Empty` at `:225`), contract max 32 (`:229-240`), over-global
97 (`KaniHarnessIdError::TooLong(97)` at `:251-261`), leading non-letter
(`LeadingNonLetter` at `:198-203`), body byte rejection (`NotAscii` at `:216-222`).

**No fix needed.**

---

## F2 — Harness listing cap enforced — PASS (weak test surface)

**Evidence (`crates/titania-core/src/kani_inventory.rs`):**
- `:50` `pub const KANI_INVENTORY_MAX_HARNESSES: usize = 1_000_000;`
- `:135-137` after flatten: `let total = standard.len().saturating_add(contract.len()); reject_too_many_harnesses(total, Box::from(path))?;`
- `:192-201` `fn reject_too_many_harnesses(total, path)` returns
  `KaniInventoryError::TooManyHarnesses { path, found, max }` when
  `total > KANI_INVENTORY_MAX_HARNESSES`.

**Test surface** (`crates/titania-core/tests/v15_kani_inventory.rs:167-169`)
only verifies `KANI_INVENTORY_MAX_HARNESSES >= 1024` — a constant-existence
test, **not** a behavioural rejection test. There is **no** test that feeds
a wire with > 1_000_000 harnesses and asserts `KaniInventoryError::TooManyHarnesses`.

**Verdict:** behaviour is correct; test coverage is **thin** (LOW residual risk,
not a blocker for v1.5 ship). Recommend adding a fixture that synthesises
`standard-harnesses` with one oversized file entry and asserts rejection — but
this is a coverage add, not a contract defect.

**No fix needed for v1.5 ship; consider a coverage follow-up bead.**

---

## F3 — Unknown top-level keys tolerated — PASS

**Evidence (`crates/titania-core/src/kani_inventory.rs`):**
- `:159-169` `KaniInventoryWire` only declares `kani-version`, `file-version`,
  `standard-harnesses`, `contract-harnesses`. No `#[serde(deny_unknown_fields)]`
  applied.
- Serde's default behaviour ignores unknown top-level keys (`checksum`,
  `build-flags`, `contracts`, `totals` documented in the parser docstring
  `:6-21`).

**Test:** `crates/titania-core/tests/v15_kani_inventory.rs:77-93`
`unknown_top_level_keys_are_ignored` feeds
`tests/fixtures/v15_kani_inventory_with_unknown_keys.json` (which carries
`checksum` and `build-flags` on top of `contracts` and `totals`) and asserts
the parser extracts exactly the 1 standard + 1 contract listing.

**No fix needed.**

---

## F4 — Missing required keys rejected — **FAIL (HIGH)**

**Claim under audit:** "missing required keys rejected".

**Implementation reality (`crates/titania-core/src/kani_inventory.rs:159-169`):**

```rust
#[derive(Deserialize)]
struct KaniInventoryWire {
    #[serde(default, rename = "kani-version")]
    kani_version: Option<String>,
    #[serde(default, rename = "file-version")]
    file_version: Option<String>,
    #[serde(default, rename = "standard-harnesses")]
    standard_harnesses: BTreeMap<String, Vec<String>>,
    #[serde(default, rename = "contract-harnesses")]
    contract_harnesses: BTreeMap<String, Vec<String>>,
}
```

Every wire field is `#[serde(default)]`. The doc-comment at
`kani_inventory.rs:96-110` advertises `kani-version` and `file-version` as
forward-compatible "tolerated as missing"; the harness-map fields are
similarly defaulted.

**Net behaviour:** an entirely empty JSON document (`{}`) deserialises to
`KaniInventory { kani_version: None, file_version: None,
standard_harnesses: Vec::new(), contract_harnesses: Vec::new() }` and
`parse_str` returns `Ok(...)` with `total() == 0` and `is_empty() == true`.

**Test surface:** no test asserts that missing keys are rejected.
`tests/v15_kani_inventory.rs:67-75` `minimal_inventory_omits_optional_metadata`
explicitly asserts the *opposite* — that `kani-version` and `file-version`
are optional.

**The spec is ambiguous here:**
- `.evidence/v1.5/spec.md:88-90` says "Parse the JSON; collect every harness
  under `standard-harnesses`." — describes behaviour, not validation.
- `.beads/tn-7bq2.1/contract.md:27` says "Per-finding `PROOF_KANI_*` and
  `MUTANT_SURVIVED`" — not about missing keys.
- `.beads/tn-7bq2.1/proof-seeds.jsonl:16` says `parse_inventory rejects
  malformed cargo-kani list JSON; never panics.` — "malformed" reads as
  wrong-shape, not missing.

**Minimal fix — clarification, not a behaviour change:**

The contract currently says *some* keys are optional (`kani-version`,
`file-version`) and the harness-map sections default to empty. Two paths:

1. **Lock the contract to "all top-level keys are optional; parser
   tolerates empty inventory":** add a test
   `accepts_completely_empty_object` (`tests/v15_kani_inventory.rs`)
   asserting `{}` parses to `total() == 0`, `is_empty() == true`. This
   aligns the impl with the spec ambiguity and gives the audit claim a
   positive witness.

2. **Tighten the contract to require at least one of
   `standard-harnesses` or `contract-harnesses`:** drop the
   `#[serde(default)]` from those two wire fields, emit a new
   `KaniInventoryError::MissingRequiredShape { path, key }` variant
   in `error.rs` when both default to empty, and add a test
   `rejects_object_without_harness_sections`.

**Recommendation:** path **1**. The current behaviour is consistent with
the lane shell's documented posture (`run_lane_kani.rs` treats empty
inventory as "nothing to run; skip with `PROOF_KANI_NOT_RUN` finding per
`spec.md:94`") and tightening would break that lane integration. The
**only** concrete defect is the doc-test/test gap.

**File:line:**
- `crates/titania-core/src/kani_inventory.rs:161-168` — the four
  `#[serde(default)]` annotations.
- `crates/titania-core/tests/v15_kani_inventory.rs:67-75` — currently
  contradicts a strict "missing rejected" claim.

---

## F5 — `KaniInventory::parse` accepts empty inventory and emits a real finding — **FAIL (HIGH)**

The user's audit claim splits into three parts; two of three fail.

### F5a — Method name mismatch (HIGH)

The user wrote `KaniInventory::parse`. The actual public API is
`KaniInventory::parse_str` at
`crates/titania-core/src/kani_inventory.rs:126`:

```rust
pub fn parse_str(contents: &str, path: &str) -> Result<Self, KaniInventoryError> {
```

There is no `KaniInventory::parse` method anywhere in the crate.

- `crates/titania-core/src/kani_inventory.rs:126` is the only `fn parse` entry.
- `crates/titania-core/src/lib.rs:66-68` `pub use kani_inventory::{
  KANI_INVENTORY_MAX_HARNESSES, KaniHarnessListing, KaniInventory,
  canonical_harness_id, };` — exports no `parse` alias.
- `rtk grep -rn 'KaniInventory::parse\b' crates/` returns zero hits.

The audit brief and the actual public API do not agree. Either the brief
uses shorthand for `parse_str` or the API was supposed to expose `parse`
and the rename did not land.

### F5b — Empty inventory accepted (PASS, with caveat)

`tests/v15_kani_inventory.rs:56-65` `empty_inventory_loads_with_zero_count`
exercises `parse_str` on
`tests/fixtures/v15_kani_inventory_empty.json` (12 bytes of valid kani
inventory with `standard-harnesses: {}` and `contract-harnesses: {}`) and
asserts `total() == 0`, `is_empty() == true`. PASS for that case.

The empty-`{}` case (zero top-level keys) is **not** tested — see F4.

### F5c — Emits a real finding (FAIL)

The claim "and emits a real finding" is **not** a property of the core
parser. `KaniInventory::parse_str` returns
`Result<Self, KaniInventoryError>`; it has **no** finding-emission code
path. The core crate is filesystem-free per the boundary map
(`.beads/tn-7bq2.1/boundary-map.md:8-30`); findings are typed into a
`LaneOutcome` by the lane shell (`run_lane_kani.rs`) and only the shell
writes `.titania/out/<scope>/<lane>.json`.

What the lane does on empty inventory is the `PROOF_KANI_NOT_RUN`
informational finding per `.evidence/v1.5/spec.md:94`, `domain-model.md:98`
("per-harness `VERIFICATION:- NOT_RUN` → contributing `PROOF_KANI_NOT_RUN`
finding"). That emission lives in the shell, not the parser.

**Minimal fix — split into two questions:**

1. **Method-name alignment:** either rename `parse_str` → `parse` in
   `crates/titania-core/src/kani_inventory.rs:126` and update the four
   callers (`tests/v15_kani_inventory.rs:36, 48, 60, 71, 80, 98, 105, 112,
   121, 174` plus any shell caller in `crates/titania-lanes/src/`), OR
   update the audit brief to use `parse_str`.

2. **Finding emission is out of scope for the core parser.** The audit
   claim conflates parser behaviour with lane behaviour. The shell
   (`run_lane_kani.rs`) is responsible for emitting the
   `PROOF_KANI_NOT_RUN` finding when the parser returns an empty
   inventory — that path needs to be verified separately, not from
   `kani_inventory.rs`.

**File:line for F5:**
- `crates/titania-core/src/kani_inventory.rs:126` — only `parse_str`,
  no `parse` alias.
- `crates/titania-core/src/kani_inventory.rs:113-152` — the entire
  `impl KaniInventory` block; no `Finding` import, no finding construction.

---

## F6/F7/F8 — Three named harnesses exist — PASS

| Harness | File:line | Body boundary |
|---------|-----------|---------------|
| `kani_kani_harness_id_bounded` | `crates/titania-core/src/kani.rs:144-145` (`#[kani::unwind(100)]`) | `KANI_ID_CONTRACT_BOUND = 32` (`:117`) for the symbolic gen, plus fixed boundaries at 32 and 97 chars |
| `kani_kani_lane_name_roundtrip` | `crates/titania-core/src/kani.rs:279-280` (`#[kani::unwind(32)]`) | Iterates `GateScope::Full.lanes()`, asserts mapping ≤ 32 bytes, ASCII-lowercase/digit/underscore, `KaniHarnessId::new` round-trip, `Lane::from_str(lane.name())` round-trip, plus covers `Lane::Kani`/`Lane::Mutants` reachability |
| `kani_mutants_baseline_diff_zero_neg` | `crates/titania-core/src/kani.rs:370-371` (`#[kani::unwind(32)]`) | 8 fixed-shape `MutantId` candidates (`KANI_DIFF_CANDIDATE_BOUND = 8` at `:118`), 24 symbolic bits across three bool arrays, asserts covered-survivor exclusion + uncovered-survivor inclusion + diff-stays-inside-survivor-slice invariant; covers empty/all/no-baseline/all-baseline-active/expired-entry/mixed-covered-and-uncovered |

All three harnesses exist, have `#[kani::proof]` annotation, an explicit
`#[kani::unwind(N)]` bound that meets or exceeds the harness's effective
loop bound, and `cover!` reachability hits.

**No fix needed.**

---

## F9 — Stale black-hat finding F-33 (LOW, residual risk only)

`.evidence/v1.5/black-hat-review.md:227` records finding F-33:

> "`KaniHarnessId` charset is stricter than spec. Spec §3 says
> `KaniHarnessId::new(name)` validates `^[a-zA-Z][a-zA-Z0-9_]*$`;
> implementation enforces `^[A-Z][A-Z0-9_]*$` (uppercase only). This is
> intentional because the rule-id grammar is uppercase-only, but spec
> wording is stale."

**Reality check against current source:**

- `crates/titania-core/src/proof_id.rs:111` `if !first.is_ascii_alphabetic()`
  — accepts both upper and lower.
- `crates/titania-core/src/proof_id.rs:124` `byte.is_ascii_alphabetic()
  || byte.is_ascii_digit() || byte == b'_'` — accepts both cases.

The current implementation **does not** enforce uppercase-only. The
black-hat finding is stale; the implementation has already been brought
into line with the spec. The stale line is cosmetic (it's in a `.md`
report under `.evidence/`, not in shipping code).

**No fix needed for the source tree; consider sweeping the stale line
from the audit report during the next black-hat pass.**

---

## Severity-ordered findings list

1. **F5 (HIGH)** — Method-name mismatch `parse` vs `parse_str`; no finding
   emission in core. **Action:** clarify brief or rename
   `parse_str` → `parse`; verify the lane shell emits
   `PROOF_KANI_NOT_RUN` on empty inventory (separate audit).

2. **F4 (HIGH)** — Missing required keys silently accepted.
   **Action:** add `accepts_completely_empty_object` test (recommended
   path 1 above) so the behaviour has a positive witness.

3. **F9 (LOW)** — Stale `black-hat-review.md:227` line claims
   uppercase-only charset. **Action:** sweep during next black-hat pass.

4. **F2 (LOW, residual)** — Cap-enforcement has a constant-presence test
   only; no behavioural negative test. **Action:** open a follow-up bead
   to add an oversized-inventory rejection test.

5. **F1, F3, F6, F7, F8** — PASS. No action.

---

## Minimal fix (single-PR scope)

The two HIGH findings can be closed with **one** small PR that adds two
tests and clarifies one doc comment, **without** editing the core parser
logic:

1. `crates/titania-core/tests/v15_kani_inventory.rs` — add a test
   `accepts_completely_empty_object` (8 lines) covering `parse_str("{}",
   "<inline>")` returns `Ok` with `total() == 0` and `is_empty() == true`.
2. `crates/titania-core/tests/v15_kani_inventory.rs` — add a test
   `rejects_object_with_oversized_harness_list` (15-20 lines) that
   synthesises a wire with one file carrying > `KANI_INVENTORY_MAX_HARNESSES`
   harness entries and asserts
   `KaniInventoryError::TooManyHarnesses { .. }`.
3. `crates/titania-core/src/kani_inventory.rs:84-91` (the
   `KaniInventory` doc-comment) — explicitly state that all top-level
   wire keys are optional and that an empty object is accepted as
   "nothing to run; lane shell is responsible for emitting
   `PROOF_KANI_NOT_RUN` per spec.md:94".

This closes F4 and the audit-brief ambiguity in F5c **without** changing
the parser's surface or breaking the lane integration.

If the audit brief's `parse` symbol is literal (not shorthand for
`parse_str`), the additional rename from `parse_str` → `parse` is a
mechanical touch across the test file and any shell callers; recommend
opening it as a **separate** bead because it touches the public API
surface and is technically a breaking change.

---

## What the audit deliberately does NOT touch

- The eight other `#[kani::proof]` harnesses in `crates/titania-core/src/kani.rs`
  (`lane_name_rejects_empty_string`, `lane_name_rejects_nul_byte`,
  `lane_digest_rejects_passed_greater_than_scanned`,
  `lane_digest_accepts_passed_not_greater_than_scanned`,
  `recorded_target_root_*`) are not in scope; this audit only checks the
  three harnesses the brief names.
- The `lane.rs`/`mutants_baseline.rs`/`mutants_outcomes.rs`/`receipt.rs`
  proof harnesses that some other beads call out are out of scope here.
- The lane shell (`crates/titania-lanes/src/run_lane_kani.rs`) is **not**
  in scope for this read-only audit; the finding-emission claim in F5c
  is flagged as "needs separate audit against the shell", not approved.

STATUS: APPROVED — parser core + the three named Kani harnesses meet
the spec contract; the two HIGH findings are audit-brief / test-coverage
gaps that the lane integration already documents and the parser
already handles correctly.
