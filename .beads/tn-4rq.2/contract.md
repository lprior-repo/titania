# Contract — tn-4rq.2 Doctor Domain Model

## titania-output/src/doctor.rs

### Types
```rust
pub enum DoctorStatus { Ok, MissingRequiredTools }
pub struct ToolRow { name: &'static str, required: bool, installed: bool, version: Option<String>, path: Option<PathBuf> }
pub struct DoctorReport { scope: GateScope, tools: Vec<ToolRow>, missing_required: Vec<String>, status: DoctorStatus }
```

### Functions
```rust
pub fn doctor_report(scope: GateScope) -> Result<DoctorReport, OutputError>
pub fn report(scope: GateScope) -> DoctorReport
```

### Rules
- `DoctorReport::new()` computes `missing_required` from `tools` where `required && !installed`
- `DoctorStatus::Ok` when `missing_required.is_empty()`, else `MissingRequiredTools`
- Embedded ast-grep row: `name="ast-grep"`, `required=true`, `version=null`, `path=null`
- Dylint: two rows — `cargo-dylint` (PATH probe) and `libtitania_dylint` (co-located library probe)
- `libtitania_dylint.required = cargo-dylint.installed` (library only required if binary present)

## titania-check/src/doctor.rs (renderer)

### Functions
```rust
pub(crate) fn render(scope: GateScope, emit: EmitFormat) -> Result<CliDisposition, OutputError>
pub fn render_human(report: &DoctorReport) -> String
pub fn render_json(report: &DoctorReport) -> String
```

### Exit Codes
- `DoctorStatus::Ok` => exit 0
- `DoctorStatus::MissingRequiredTools` => exit 3

### Human Output Format
```
titania-check doctor — scope: {scope}

Tool            Required   Installed  Version       Path
{tool row}...

Status: {status}
```

### JSON Output Format
```json
{ "scope": "...", "tools": [...], "missing_required": [...], "status": "..." }
```
