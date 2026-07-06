# Test Plan — tn-4rq.2 Doctor

## Unit Tests (titania-output)
- [x] DoctorStatus::as_str returns "OK" / "MissingRequiredTools"
- [x] ToolRow::new and ToolRow::embedded construction
- [x] DoctorReport::new computes missing_required and status correctly
- [x] Scope-based tool config: edit has optional cargo-deny/sccache, prepush/release has required cargo-deny

## CLI Integration Tests (titania-check/tests/doctor.rs)
### Human Output
- [x] `doctor --scope edit` outputs human table with scope header
- [x] Human output contains Tool/Required/Installed/Version/Path column headers
- [x] Human output contains required tool rows (cargo, rustfmt, clippy-driver, rg, ast-grep, cargo-dylint)
- [x] Human output contains Status: line with OK or MissingRequiredTools
- [x] Prepush and release scope headers present

### JSON Output
- [x] `doctor --emit json` outputs parseable JSON
- [x] JSON contains scope field matching requested scope
- [x] JSON contains tools array with entries
- [x] Each tool has name (string), required (bool), installed (bool)
- [x] JSON contains missing_required array
- [x] JSON status is "OK" or "MissingRequiredTools"
- [x] cargo-deny required=false for edit scope
- [x] cargo-deny required=true for prepush scope
- [x] Embedded ast-grep: required=true, version=null, path=null
- [x] Dylint rows present: cargo-dylint and libtitania_dylint

### Exit Codes
- [x] Missing required tools => exit code 3 (PATH-empty test)
- [x] Missing required tools => JSON status "MissingRequiredTools"
- [x] Optional sccache missing does not force MissingRequiredTools
- [x] Unknown scope => exit code 3 (InputError)
- [x] Default scope is edit

## CLI Dispatch Tests (cli_dispatch.rs)
- [x] dispatch_doctor_emits_human_table — real doctor output
- [x] exit_codes_doctor_emits_json — real doctor JSON
- [x] cli_args_dispatch_missing_implementation_exit_codes — updated to real doctor

## Non-Goals (not tested here)
- Template work, release packaging, docs beyond artifacts
- Moon task config
- Dylint ABI mismatch detection (requires actual ABI mismatch scenario)
