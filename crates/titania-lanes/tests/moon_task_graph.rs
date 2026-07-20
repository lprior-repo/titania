//! Integration test for `.moon/tasks/all.yml` — v1/v1.5 task DAG.
//!
//! Asserts the exact task contract for all 16 required tasks (12 lane +
//! 4 gate composites) co-exist with legacy cargo-native tasks. The test
//! intentionally fails RED when v1 tasks are absent, proving the parser and
//! assertion logic work (not a silent pass).
//!
//! Key contracts:
//!   • deps use `~:name` (Moon local task dep syntax)
//!   • gate tasks have NO toolchains key
//!   • compile fan-out: clippy, test, build depend on `~:titania-compile`
//!   • every Full lane declares `.titania/out/full/<lane>.json`
//!   • heavy build/dylint/Kani/mutants lanes use distinct target dirs
//!   • env/inputs/outputs are declared exactly

use std::collections::HashMap;

use anyhow::{Context, Result};

// The YAML file is at the workspace root; from tests/ go up 3 dirs.
const MOON_TASKS: &str = include_str!("../../../.moon/tasks/all.yml");

// ──────────────────────────────────────────────────────────────────────────────
// Minimal YAML line-scanner — sufficient for v1 §13 forms.
// ──────────────────────────────────────────────────────────────────────────────

/// A parsed task entry extracted from the `tasks:` map of all.yml.
#[derive(Debug)]
struct ParsedTask {
    /// Top-level key under `tasks:` (e.g. "titania-fmt").
    name: String,
    /// The `command:` scalar value (trimmed).
    command: Option<String>,
    /// Elements of `toolchains: [rust]`.
    toolchains: Vec<String>,
    /// Whether a `toolchains:` key was present in the YAML (even if empty).
    /// Distinguishes "key absent" from "key present but empty".
    toolchains_key: bool,
    /// Elements of `deps:` (both flow and block list formats).
    deps: Vec<String>,
    /// `env:` block key-value pairs.
    env: HashMap<String, String>,
    /// `script:` scalar or folded block value.
    script: Option<String>,
    /// `inputs:` block-list entries.
    inputs: Vec<String>,
    /// `outputs:` block-list entries.
    outputs: Vec<String>,
    /// `runInCI: true` inside `options:` block.
    run_in_ci: Option<bool>,
}
fn parse_task_yaml_clean(text: &str) -> Vec<ParsedTask> {
    let lines: Vec<&str> = text.lines().collect();
    let mut tasks = Vec::new();

    // Find the `tasks:` line at column 0.
    let tasks_start = match (0..lines.len()).find(|&i| lines[i].trim() == "tasks:") {
        Some(idx) => idx + 1,
        None => return tasks,
    };

    // Collect the task-map block: lines indented > 0 after `tasks:`.
    let mut block_lines: Vec<(usize, &str)> = Vec::new();
    for idx in tasks_start..lines.len() {
        let line = lines[idx];
        if line.trim().is_empty() {
            continue;
        }
        let indent = line.chars().take_while(|c| *c == ' ').count();
        if indent == 0 {
            break;
        }
        block_lines.push((idx, line));
    }

    // Split block_lines into task sub-blocks.
    // Tasks start at indent 2; sub-keys at indent 4+.
    let mut sub_blocks: Vec<Vec<(usize, &str)>> = Vec::new();
    for &(idx, line) in &block_lines {
        let trimmed = line.trim();
        let indent = line.chars().take_while(|c| *c == ' ').count();
        // Skip YAML comments and blank lines at task indent.
        if trimmed.starts_with('#') {
            continue;
        }
        if indent == 2 {
            // Start of a new task entry.
            sub_blocks.push(vec![(idx, line)]);
        } else if let Some(last) = sub_blocks.last_mut() {
            last.push((idx, line));
        }
    }

    // Parse each sub-block into a ParsedTask.
    for sub in &sub_blocks {
        let first_line = sub[0].1;
        let name = first_line.trim().strip_suffix(':').unwrap_or(first_line.trim()).to_string();

        // Scan sub-lines collecting fields; handle block-format fields inline.
        let mut command: Option<String> = None;
        let mut toolchains: Vec<String> = Vec::new();
        let mut toolchains_key: bool = false;
        let mut deps: Vec<String> = Vec::new();
        let mut env = HashMap::new();
        let mut script: Option<String> = None;
        let mut inputs: Vec<String> = Vec::new();
        let mut outputs: Vec<String> = Vec::new();
        let mut run_in_ci: Option<bool> = None;
        let mut in_options: bool = false;

        let mut i = 1;
        while i < sub.len() {
            let (_line_idx, line) = sub[i];
            let trimmed = line.trim();
            let field_indent = line.chars().take_while(|c| *c == ' ').count();

            if trimmed.is_empty() {
                i += 1;
                continue;
            }

            // Detect options: block start.
            if trimmed == "options:" {
                in_options = true;
                i += 1;
                continue;
            }
            if in_options && trimmed.starts_with("runInCI:") {
                let val = trimmed
                    .strip_prefix("runInCI:")
                    .unwrap_or("")
                    .trim()
                    .trim_matches('\'')
                    .trim_matches('"');
                run_in_ci = Some(parse_bool(val));
                i += 1;
                continue;
            }
            if in_options && !trimmed.starts_with("runInCI:") {
                // Left the options block.
                in_options = false;
            }

            // `command: '...'`
            if let Some(val) = trimmed.strip_prefix("command:") {
                command = Some(val.trim().trim_matches('\'').trim_matches('"').to_string());
            }
            // `toolchains:` key present — set flag
            // Flow array: `toolchains: [rust]` or block list: indented `- '...'` lines
            else if trimmed.starts_with("toolchains:") {
                toolchains_key = true;
                let val = trimmed.strip_prefix("toolchains:").unwrap_or("").trim();
                if val.starts_with('[') || !val.is_empty() {
                    // Flow array
                    toolchains = parse_flow_array(val);
                } else {
                    // Block list — read indented `- '...'` lines
                    i += 1;
                    while i < sub.len() {
                        let (_, next_line) = sub[i];
                        let next_indent = next_line.chars().take_while(|c| *c == ' ').count();
                        let next_trimmed = next_line.trim();
                        if next_trimmed.is_empty() {
                            i += 1;
                            continue;
                        }
                        if next_indent <= field_indent {
                            break;
                        }
                        if next_trimmed.starts_with("- ") {
                            let tc = next_trimmed
                                .strip_prefix("- ")
                                .unwrap_or(next_trimmed)
                                .trim()
                                .trim_matches('\'')
                                .trim_matches('"')
                                .to_string();
                            if !tc.is_empty() {
                                toolchains.push(tc);
                            }
                        }
                        i += 1;
                    }
                    continue;
                }
            }
            // `deps: ['~:foo', ...]` (flow array)
            else if let Some(val) = trimmed.strip_prefix("deps:") {
                let value = val.trim();
                if !value.is_empty() {
                    deps = parse_flow_deps(value);
                } else {
                    i += 1;
                    while i < sub.len() {
                        let (_, next_line) = sub[i];
                        let next_indent = next_line.chars().take_while(|c| *c == ' ').count();
                        let next_trimmed = next_line.trim();
                        if next_trimmed.is_empty() {
                            i += 1;
                            continue;
                        }
                        if next_indent <= field_indent {
                            break;
                        }
                        if next_trimmed.starts_with("- ") {
                            let dep = next_trimmed
                                .strip_prefix("- ")
                                .unwrap_or(next_trimmed)
                                .trim()
                                .trim_matches('\'')
                                .trim_matches('"')
                                .to_string();
                            deps.push(dep);
                        }
                        i += 1;
                    }
                    continue;
                }
            }
            // `script: >-` folded block or scalar script.
            else if let Some(val) = trimmed.strip_prefix("script:") {
                let raw_value = val.trim();
                if matches!(raw_value, ">-" | ">" | "|-" | "|") {
                    let (block_script, next_index) = parse_block_scalar(sub, i + 1, field_indent);
                    script = Some(block_script);
                    i = next_index;
                    continue;
                }
                script = Some(raw_value.trim_matches('\'').trim_matches('"').to_string());
            }
            // `env:` block (key: value lines)
            else if trimmed == "env:" {
                let _block_indent = field_indent + 2;
                i += 1;
                while i < sub.len() {
                    let (_, next_line) = sub[i];
                    let next_indent = next_line.chars().take_while(|c| *c == ' ').count();
                    let next_trimmed = next_line.trim();
                    if next_trimmed.is_empty() {
                        i += 1;
                        continue;
                    }
                    if next_indent <= field_indent {
                        break;
                    }
                    if let Some(kv) = next_trimmed.split_once(':') {
                        let key = kv.0.trim().to_string();
                        let val = kv.1.trim().trim_matches('\'').trim_matches('"').to_string();
                        drop(env.insert(key, val));
                    }
                    i += 1;
                }
                continue;
            }
            // `inputs:` block list
            else if trimmed == "inputs:" {
                i += 1;
                while i < sub.len() {
                    let (_, next_line) = sub[i];
                    let next_indent = next_line.chars().take_while(|c| *c == ' ').count();
                    let next_trimmed = next_line.trim();
                    if next_trimmed.is_empty() {
                        i += 1;
                        continue;
                    }
                    if next_indent <= field_indent {
                        break;
                    }
                    if next_trimmed.starts_with("- ") {
                        let item = next_trimmed
                            .strip_prefix("- ")
                            .unwrap_or(next_trimmed)
                            .trim()
                            .trim_matches('\'')
                            .trim_matches('"')
                            .to_string();
                        inputs.push(item);
                    }
                    i += 1;
                }
                continue;
            }
            // `outputs:` block list
            else if trimmed == "outputs:" {
                i += 1;
                while i < sub.len() {
                    let (_, next_line) = sub[i];
                    let next_indent = next_line.chars().take_while(|c| *c == ' ').count();
                    let next_trimmed = next_line.trim();
                    if next_trimmed.is_empty() {
                        i += 1;
                        continue;
                    }
                    if next_indent <= field_indent {
                        break;
                    }
                    if next_trimmed.starts_with("- ") {
                        let item = next_trimmed
                            .strip_prefix("- ")
                            .unwrap_or(next_trimmed)
                            .trim()
                            .trim_matches('\'')
                            .trim_matches('"')
                            .to_string();
                        outputs.push(item);
                    }
                    i += 1;
                }
                continue;
            }

            i += 1;
        }

        tasks.push(ParsedTask {
            name,
            command,
            toolchains,
            toolchains_key,
            deps,
            env,
            script,
            inputs,
            outputs,
            run_in_ci,
        });
    }

    tasks
}

fn parse_block_scalar(sub: &[(usize, &str)], start: usize, field_indent: usize) -> (String, usize) {
    let mut lines = Vec::new();
    let mut index = start;
    while index < sub.len() {
        let (_, line) = sub[index];
        let indent = line.chars().take_while(|c| *c == ' ').count();
        let trimmed = line.trim();
        if trimmed.is_empty() {
            index += 1;
            continue;
        }
        if indent <= field_indent {
            break;
        }
        lines.push(trimmed.to_string());
        index += 1;
    }
    (lines.join("\n"), index)
}

/// Parse `toolchains: [rust]` or `toolchains: [rust, ...]` flow array.
fn parse_flow_array(s: &str) -> Vec<String> {
    let inner = s.trim().strip_prefix('[').unwrap_or(s).strip_suffix(']').unwrap_or(s);
    inner
        .split(',')
        .map(|v| v.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Parse `deps: ['dep1', 'dep2']` flow array, returning dep strings.
fn parse_flow_deps(s: &str) -> Vec<String> {
    let inner = s.trim().strip_prefix('[').unwrap_or(s).strip_suffix(']').unwrap_or(s);
    inner
        .split(',')
        .map(|v| v.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|v| !v.is_empty())
        .collect()
}

/// Parse a boolean string.
fn parse_bool(s: &str) -> bool {
    matches!(s.to_lowercase().as_str(), "true" | "yes" | "1")
}

// ──────────────────────────────────────────────────────────────────────────────
// v1 §13 task contract assertions.
// ──────────────────────────────────────────────────────────────────────────────

/// All required v1/v1.5 tasks (12 lane + 4 gate composites).
const REQUIRED_TASKS: &[&str] = &[
    "titania-fmt",
    "titania-compile",
    "titania-clippy",
    "titania-ast-grep",
    "titania-dylint",
    "titania-panic-scan",
    "titania-policy-scan",
    "titania-test",
    "titania-deny",
    "titania-build",
    "titania-kani",
    "titania-mutants",
    "gate-edit",
    "gate-prepush",
    "gate-release",
    "gate-full",
];

const LANE_COMMANDS: &[(&str, &str)] = &[
    ("titania-fmt", "cargo run --frozen --quiet -p titania-check -- run-lane fmt"),
    (
        "titania-compile",
        "CARGO_TARGET_DIR=\".titania/cache/compile\" cargo run --frozen --quiet -p titania-check -- run-lane compile",
    ),
    (
        "titania-clippy",
        "CARGO_TARGET_DIR=\".titania/cache/clippy\" cargo run --frozen --quiet -p titania-check -- run-lane clippy",
    ),
    ("titania-ast-grep", "cargo run --frozen --quiet -p titania-check -- run-lane ast-grep"),
    (
        "titania-dylint",
        "PATH=\"$HOME/.local/share/mise/shims:$HOME/.cargo/bin:$PATH\" CARGO_TARGET_DIR=\".titania/cache/dylint\" cargo run --frozen --quiet -p titania-check -- run-lane dylint",
    ),
    ("titania-panic-scan", "cargo run --frozen --quiet -p titania-check -- run-lane panic-scan"),
    ("titania-policy-scan", "cargo run --frozen --quiet -p titania-check -- run-lane policy-scan"),
    (
        "titania-test",
        "PATH=\"$HOME/.local/share/mise/shims:$HOME/.cargo/bin:$PATH\" CARGO_TARGET_DIR=\".titania/cache/test\" cargo run --frozen --quiet -p titania-check -- run-lane test",
    ),
    ("titania-deny", "cargo run --frozen --quiet -p titania-check -- run-lane deny"),
    (
        "titania-build",
        "CARGO_TARGET_DIR=\".titania/cache/build\" cargo run --frozen --quiet -p titania-check -- run-lane build",
    ),
];

const LANE_SCRIPTS: &[(&str, &str)] = &[
    (
        "titania-kani",
        "CARGO_TARGET_DIR=\".titania/cache/kani\" cargo run --frozen --quiet -p titania-check -- run-lane kani",
    ),
    (
        "titania-mutants",
        "CARGO_TARGET_DIR=\".titania/cache/mutants\" cargo run --frozen --quiet -p titania-check -- run-lane mutants",
    ),
];

const HEAVY_TARGET_DIRS: &[(&str, &str)] = &[
    ("titania-build", ".titania/cache/build"),
    ("titania-dylint", ".titania/cache/dylint"),
    ("titania-kani", ".titania/cache/kani"),
    ("titania-mutants", ".titania/cache/mutants"),
];

/// Expected commands for each gate composite.
const GATE_COMMANDS: &[(&str, &str)] = &[
    ("gate-edit", "edit"),
    ("gate-prepush", "prepush"),
    ("gate-release", "release"),
    ("gate-full", "full"),
];

/// Lane tasks that MUST have `toolchains: [rust]` (§13).
const RUST_TOOLCHAIN_TASKS: &[&str] = &[
    "titania-fmt",
    "titania-compile",
    "titania-clippy",
    "titania-dylint",
    "titania-test",
    "titania-build",
    "titania-kani",
    "titania-mutants",
];

/// Lane tasks that MUST NOT have a `toolchains` key (§13).
const NO_TOOLCHAIN_TASKS: &[&str] =
    &["titania-ast-grep", "titania-panic-scan", "titania-policy-scan", "titania-deny"];

/// Gate tasks that MUST NOT have a `toolchains` key.
const GATE_NO_TOOLCHAIN_TASKS: &[&str] =
    &["gate-edit", "gate-prepush", "gate-release", "gate-full"];

/// Tasks that depend on `~:titania-compile` (§13, compile fan-out).
const COMPILE_DEPS: &[&str] = &["titania-clippy", "titania-test", "titania-build"];

/// Gate dependency maps: gate → [dep task names] (for building `~:name` strings).
const GATE_DEPS: &[(&str, &[&str])] = &[
    (
        "gate-edit",
        &[
            "titania-fmt",
            "titania-compile",
            "titania-clippy",
            "titania-ast-grep",
            "titania-dylint",
            "titania-panic-scan",
            "titania-policy-scan",
        ],
    ),
    ("gate-prepush", &["gate-edit", "titania-test", "titania-deny"]),
    ("gate-release", &["gate-prepush", "titania-build"]),
    ("gate-full", &["gate-release", "titania-kani", "titania-mutants"]),
];

/// Expected inputs per v1 lane task (§13).
const LANE_INPUTS: &[(&str, &[&str])] = &[
    (
        "titania-fmt",
        &[
            "@globs(sources)",
            "rustfmt.toml",
            ".titania/**",
            "!.titania/cache/**",
            "!.titania/out/**",
        ],
    ),
    (
        "titania-compile",
        &["@globs(sources)", "Cargo.toml", "Cargo.lock", ".cargo/**", "rust-toolchain.toml"],
    ),
    (
        "titania-clippy",
        &[
            "@globs(sources)",
            "clippy.toml",
            "Cargo.toml",
            ".titania/**",
            "!.titania/cache/**",
            "!.titania/out/**",
        ],
    ),
    ("titania-ast-grep", &["@globs(sources)"]),
    ("titania-dylint", &["@globs(sources)"]),
    ("titania-panic-scan", &["@globs(sources)"]),
    (
        "titania-policy-scan",
        &[
            "@globs(sources)",
            "Cargo.toml",
            "**/Cargo.toml",
            ".cargo/**",
            ".titania/**",
            "!.titania/cache/**",
            "!.titania/out/**",
        ],
    ),
    ("titania-test", &["@globs(sources)", "@globs(tests)"]),
    ("titania-deny", &["@globs(sources)", "Cargo.lock", "deny.toml"]),
    ("titania-build", &["@globs(sources)"]),
    (
        "titania-kani",
        &[
            "@globs(sources)",
            "Cargo.toml",
            "Cargo.lock",
            ".titania/profiles/strict-ai/mutants.baseline.json",
        ],
    ),
    (
        "titania-mutants",
        &[
            "@globs(sources)",
            "Cargo.toml",
            "Cargo.lock",
            ".titania/profiles/strict-ai/mutants.baseline.json",
        ],
    ),
];

/// Expected outputs per lane task.
const LANE_OUTPUTS: &[(&str, &[&str])] = &[
    (
        "titania-fmt",
        &[
            ".titania/out/edit/fmt.json",
            ".titania/out/prepush/fmt.json",
            ".titania/out/release/fmt.json",
            ".titania/out/full/fmt.json",
        ],
    ),
    (
        "titania-compile",
        &[
            ".titania/out/edit/compile.json",
            ".titania/out/prepush/compile.json",
            ".titania/out/release/compile.json",
            ".titania/out/full/compile.json",
        ],
    ),
    (
        "titania-clippy",
        &[
            ".titania/out/edit/clippy.json",
            ".titania/out/prepush/clippy.json",
            ".titania/out/release/clippy.json",
            ".titania/out/full/clippy.json",
        ],
    ),
    (
        "titania-ast-grep",
        &[
            ".titania/out/edit/ast-grep.json",
            ".titania/out/prepush/ast-grep.json",
            ".titania/out/release/ast-grep.json",
            ".titania/out/full/ast-grep.json",
        ],
    ),
    (
        "titania-dylint",
        &[
            ".titania/out/edit/dylint.json",
            ".titania/out/prepush/dylint.json",
            ".titania/out/release/dylint.json",
            ".titania/out/full/dylint.json",
        ],
    ),
    (
        "titania-panic-scan",
        &[
            ".titania/out/edit/panic-scan.json",
            ".titania/out/prepush/panic-scan.json",
            ".titania/out/release/panic-scan.json",
            ".titania/out/full/panic-scan.json",
        ],
    ),
    (
        "titania-policy-scan",
        &[
            ".titania/out/edit/policy-scan.json",
            ".titania/out/prepush/policy-scan.json",
            ".titania/out/release/policy-scan.json",
            ".titania/out/full/policy-scan.json",
        ],
    ),
    (
        "titania-test",
        &[
            ".titania/out/prepush/test.json",
            ".titania/out/release/test.json",
            ".titania/out/full/test.json",
        ],
    ),
    (
        "titania-deny",
        &[
            ".titania/out/prepush/deny.json",
            ".titania/out/release/deny.json",
            ".titania/out/full/deny.json",
        ],
    ),
    ("titania-build", &[".titania/out/release/build.json", ".titania/out/full/build.json"]),
    ("titania-kani", &[".titania/out/full/kani.json"]),
    ("titania-mutants", &[".titania/out/full/mutants.json"]),
];

/// Lane tasks keep cache/PATH exports inline so Moon shell expansion, not YAML
/// env interpolation, controls target directories and rustup availability.
const LANE_EMPTY_ENV: &[&str] = &[
    "titania-compile",
    "titania-clippy",
    "titania-dylint",
    "titania-test",
    "titania-build",
    "titania-kani",
    "titania-mutants",
];

const VET_INPUTS: &[&str] = &["Cargo.lock", "supply-chain/**"];

const GEIGER_FIRST_PARTY_MANIFESTS: &[&str] = &[
    "crates/titania-aggregate/Cargo.toml",
    "crates/titania-check/Cargo.toml",
    "crates/titania-core/Cargo.toml",
    "crates/titania-lanes/Cargo.toml",
    "crates/titania-output/Cargo.toml",
    "crates/titania-policy/Cargo.toml",
];

#[test]
fn v1_moon_tasks() -> Result<()> {
    // ── 0. Verify the include_str! loaded something ────────────────────────
    assert!(
        !MOON_TASKS.is_empty(),
        "include_str!(../../../.moon/tasks/all.yml) returned empty string"
    );

    // ── 1. Parse the task map ──────────────────────────────────────────────
    let tasks = parse_task_yaml_clean(MOON_TASKS);

    // ── 2. Assert file-level structure ─────────────────────────────────────
    assert!(MOON_TASKS.contains("\ntasks:\n"), "all.yml must define the top-level `tasks:` map");
    assert!(
        MOON_TASKS.contains("    - '.titania/profiles/**'"),
        "sources fileGroup must hash stable Titania policy profiles"
    );
    // Inputs must NOT hash runtime cache/output paths (they are generated
    // by lane runs, not sources). Negative globs like `!.titania/cache/**`
    // are allowed because they EXCLUDE the path from input hashing.
    // Outputs MAY reference runtime paths so Moon can cache the produced
    // artifacts. Distinguish inputs from outputs by tracking the
    // current section under each task, resetting on a new task or a
    // top-level key.
    let mut in_outputs_section = false;
    let mut in_inputs_section = false;
    for line in MOON_TASKS.lines() {
        let trimmed = line.trim_start();
        // A new task (2-space indent + "name:") resets section state.
        if line.starts_with("  ")
            && !line.starts_with("    ")
            && trimmed.ends_with(":")
            && !trimmed.contains(" ")
        {
            in_outputs_section = false;
            in_inputs_section = false;
        }
        if trimmed == "inputs:" {
            in_inputs_section = true;
            in_outputs_section = false;
            continue;
        }
        if trimmed == "outputs:" {
            in_outputs_section = true;
            in_inputs_section = false;
            continue;
        }
        if in_outputs_section {
            continue;
        }
        if !in_inputs_section {
            continue;
        }
        if !trimmed.starts_with("- '") {
            continue;
        }
        if trimmed.starts_with("- '!.") {
            continue;
        }
        assert!(
            !trimmed.contains(".titania/cache") && !trimmed.contains(".titania/out"),
            "Moon fileGroups/task inputs must not include Titania runtime paths: {trimmed}"
        );
    }
    // ── 3. Assert every required v1 task name exists in the parsed map ────
    let parsed_names: Vec<&str> = tasks.iter().map(|t| t.name.as_str()).collect();
    for required in REQUIRED_TASKS {
        assert!(
            parsed_names.iter().any(|n| *n == *required),
            "task '{required}' must be present in the `tasks:` map (found {parsed_names:?})"
        );
    }

    // ── 4. Build lookup and assert lane commands ───────────────────────────
    let task_map: std::collections::HashMap<&str, &ParsedTask> =
        tasks.iter().map(|t| (t.name.as_str(), t)).collect();

    let vet_task = task_map.get("vet").context("root Moon task 'vet' must be present")?;
    assert_eq!(
        vet_task.command.as_deref(),
        Some("cargo vet"),
        "vet task must run cargo vet exactly (got: {:?})",
        vet_task.command
    );
    assert!(vet_task.toolchains_key, "vet task must declare a Rust toolchain");
    assert_eq!(
        vet_task.toolchains.as_slice(),
        &["rust"],
        "vet task must use exactly toolchains: [rust]"
    );
    assert_eq!(vet_task.run_in_ci, Some(true), "vet task must have options.runInCI: true");
    assert_eq!(
        vet_task.inputs.as_slice(),
        VET_INPUTS,
        "vet task must hash exactly Cargo.lock and supply-chain audits"
    );

    let geiger_task = task_map.get("geiger").context("root Moon task 'geiger' must be present")?;
    let geiger_script = geiger_task.script.as_deref().context("geiger task must use script")?;
    for manifest in GEIGER_FIRST_PARTY_MANIFESTS {
        let expected = format!("--manifest-path \"$MOON_WORKSPACE_ROOT/{manifest}\"");
        assert!(
            geiger_script.contains(&expected),
            "geiger script must inspect first-party manifest {manifest}; script:\n{geiger_script}"
        );
    }
    assert_eq!(
        geiger_script.matches("--forbid-only").count(),
        GEIGER_FIRST_PARTY_MANIFESTS.len(),
        "each first-party cargo-geiger invocation must include --forbid-only; script:\n{geiger_script}"
    );

    for composite in ["ci", "pre-push"] {
        let task = task_map
            .get(composite)
            .with_context(|| format!("root Moon composite task '{composite}' must be present"))?;
        for dep in ["~:vet", "~:geiger"] {
            assert!(
                task.deps.contains(&dep.to_string()),
                "root Moon composite task '{composite}' must depend on {dep} (got: {:?})",
                task.deps
            );
        }
    }

    for (task_name, expected_cmd) in LANE_COMMANDS {
        let task = task_map
            .get(*task_name)
            .with_context(|| format!("lane task '{task_name}' not found in parsed tasks"))?;
        assert_eq!(
            task.command.as_deref(),
            Some(*expected_cmd),
            "lane '{task_name}' must have command '{expected_cmd}' (got: {:?})",
            task.command
        );
    }

    for (task_name, expected_script) in LANE_SCRIPTS {
        let task = task_map
            .get(*task_name)
            .with_context(|| format!("lane task '{task_name}' not found in parsed tasks"))?;
        assert_eq!(
            task.script.as_deref(),
            Some(*expected_script),
            "lane '{task_name}' must have script '{expected_script}' (got: {:?})",
            task.script
        );
    }

    for (task_name, target_dir) in HEAVY_TARGET_DIRS {
        let task = task_map
            .get(*task_name)
            .with_context(|| format!("heavy lane task '{task_name}' not found"))?;
        let execution = task
            .command
            .as_deref()
            .or(task.script.as_deref())
            .with_context(|| format!("heavy lane task '{task_name}' must be executable"))?;
        let expected = format!("CARGO_TARGET_DIR=\"{target_dir}\"");
        assert!(
            execution.contains(&expected),
            "heavy lane '{task_name}' must isolate Cargo output in {target_dir}; got: {execution}"
        );
        assert_eq!(
            execution.matches("CARGO_TARGET_DIR=").count(),
            1,
            "heavy lane '{task_name}' must set exactly one target directory; got: {execution}"
        );
    }
    assert_eq!(
        HEAVY_TARGET_DIRS
            .iter()
            .map(|(_, target_dir)| *target_dir)
            .collect::<std::collections::HashSet<_>>()
            .len(),
        HEAVY_TARGET_DIRS.len(),
        "build, dylint, Kani, and mutants target directories must be pairwise distinct"
    );

    let policy_scan =
        task_map.get("titania-policy-scan").context("lane task 'titania-policy-scan' not found")?;
    assert_eq!(
        policy_scan.env.get("CARGO_HOME").map(String::as_str),
        Some("$workspaceRoot/.titania/hermetic/cargo-home")
    );
    assert_eq!(
        policy_scan.env.get("RUSTUP_HOME").map(String::as_str),
        Some("$workspaceRoot/.titania/hermetic/rustup-home")
    );
    assert!(
        !MOON_TASKS.contains("${workspace.root}"),
        "unsupported workspace.root interpolation must not appear"
    );

    // ── 5. Assert gate commands ────────────────────────────────────────────
    for (gate_name, scope) in GATE_COMMANDS {
        let task = task_map
            .get(*gate_name)
            .with_context(|| format!("gate task '{gate_name}' not found in parsed tasks"))?;
        let expected_cmd =
            format!("cargo run --frozen --quiet -p titania-check -- aggregate --scope {scope}");
        assert_eq!(
            task.command.as_deref(),
            Some(expected_cmd.as_str()),
            "gate '{gate_name}' must have command '{expected_cmd}' (got: {:?})",
            task.command
        );
    }

    // ── 6. Assert toolchains: [rust] for lane tasks that require it ────────
    for task_name in RUST_TOOLCHAIN_TASKS {
        let task =
            task_map.get(*task_name).with_context(|| format!("task '{task_name}' not found"))?;
        assert!(
            task.toolchains_key,
            "lane task '{task_name}' must have a `toolchains:` key present (got: {:?})",
            task.toolchains
        );
        assert_eq!(
            task.toolchains.as_slice(),
            &["rust"],
            "lane task '{task_name}' must have `toolchains: [rust]` (got: {:?})",
            task.toolchains
        );
    }

    // ── 7. Assert lane tasks that must NOT have toolchains key ──────────────
    for task_name in NO_TOOLCHAIN_TASKS {
        let task =
            task_map.get(*task_name).with_context(|| format!("task '{task_name}' not found"))?;
        assert!(
            !task.toolchains_key,
            "lane task '{task_name}' must NOT have a `toolchains:` key (got: {:?})",
            task.toolchains
        );
    }
    // ── 8. Assert gate tasks have NO toolchains key ────────────────────────
    for task_name in GATE_NO_TOOLCHAIN_TASKS {
        let task = task_map
            .get(*task_name)
            .with_context(|| format!("gate task '{task_name}' not found"))?;
        assert!(
            !task.toolchains_key,
            "gate task '{task_name}' must NOT have a `toolchains:` key (got: {:?})",
            task.toolchains
        );
    }

    // ── 9. Assert compile fan-out: clippy/test/build depend on exactly `~:titania-compile` ──
    for task_name in COMPILE_DEPS {
        let task =
            task_map.get(*task_name).with_context(|| format!("task '{task_name}' not found"))?;
        let expected = vec!["~:titania-compile".to_string()];
        assert_eq!(
            task.deps, expected,
            "task '{task_name}' must have exactly `~:titania-compile` dep (got: {:?})",
            task.deps
        );
    }

    // ── 10. Assert gate deps use `~:name` prefix (Moon local dep syntax) ──
    for (gate_name, expected_deps) in GATE_DEPS {
        let task = task_map
            .get(*gate_name)
            .with_context(|| format!("gate task '{gate_name}' not found"))?;
        let expected_dep_strings: Vec<String> =
            expected_deps.iter().map(|d| format!("~:{d}")).collect();
        for dep in &expected_dep_strings {
            assert!(
                task.deps.contains(dep),
                "gate '{gate_name}' deps must include '{dep}' (got: {:?})",
                task.deps
            );
        }
        assert_eq!(
            task.deps.len(),
            expected_deps.len(),
            "gate '{gate_name}' must have exactly {} deps (got {}): {:?}",
            expected_deps.len(),
            task.deps.len(),
            task.deps
        );
    }
    // ── 11. Assert gate tasks have empty env, inputs, outputs ──────────────
    for task_name in GATE_NO_TOOLCHAIN_TASKS {
        let task = task_map
            .get(*task_name)
            .with_context(|| format!("gate task '{task_name}' not found"))?;
        assert!(
            task.env.is_empty(),
            "gate '{task_name}' must have NO env vars (got: {:?})",
            task.env
        );
        assert!(
            task.inputs.is_empty(),
            "gate '{task_name}' must have NO inputs (got: {:?})",
            task.inputs
        );
        assert!(
            task.outputs.is_empty(),
            "gate '{task_name}' must have NO outputs (got: {:?})",
            task.outputs
        );
    }

    // ── 12. Assert every required task has a command or script ─────────────
    for task_name in REQUIRED_TASKS {
        let task =
            task_map.get(*task_name).with_context(|| format!("task '{task_name}' not found"))?;
        assert!(
            task.command.is_some() || task.script.is_some(),
            "task '{task_name}' must have a `command:` or `script:` field"
        );
    }

    // ── 13. Assert cache/PATH exports are inline, not unexpanded YAML env ───
    for task_name in LANE_EMPTY_ENV {
        let task =
            task_map.get(*task_name).with_context(|| format!("task '{task_name}' not found"))?;
        assert!(
            task.env.is_empty(),
            "task '{task_name}' must use inline shell exports, not YAML env vars (got: {:?})",
            task.env
        );
    }

    // ── 14. Assert inputs per lane task (§13) ──────────────────────────────
    for (task_name, expected_inputs) in LANE_INPUTS {
        let task =
            task_map.get(*task_name).with_context(|| format!("task '{task_name}' not found"))?;
        for expected in *expected_inputs {
            assert!(
                task.inputs.contains(&expected.to_string()),
                "task '{task_name}' inputs must include '{expected}' (got: {:?})",
                task.inputs
            );
        }
        assert!(
            task.inputs
                .iter()
                .all(|input| !input.starts_with(".titania/cache")
                    && !input.starts_with(".titania/out")),
            "task '{task_name}' inputs must not hash volatile Titania output/cache paths (got: {:?})",
            task.inputs
        );
        assert_eq!(
            task.inputs.len(),
            expected_inputs.len(),
            "task '{task_name}' must have exactly {} inputs (got {}): {:?}",
            expected_inputs.len(),
            task.inputs.len(),
            task.inputs
        );
    }

    // ── 15. Assert outputs per lane task (§13) ─────────────────────────────
    for (task_name, expected_outputs) in LANE_OUTPUTS {
        let task =
            task_map.get(*task_name).with_context(|| format!("task '{task_name}' not found"))?;
        for expected in *expected_outputs {
            assert!(
                task.outputs.contains(&expected.to_string()),
                "task '{task_name}' outputs must include '{expected}' (got: {:?})",
                task.outputs
            );
        }
        assert_eq!(
            task.outputs.len(),
            expected_outputs.len(),
            "task '{task_name}' must have exactly {} outputs (got {}): {:?}",
            expected_outputs.len(),
            task.outputs.len(),
            task.outputs
        );
    }

    // ── 16. Assert runInCI: true for every required task ───────────────────
    for task_name in REQUIRED_TASKS {
        let task =
            task_map.get(*task_name).with_context(|| format!("task '{task_name}' not found"))?;
        assert!(
            task.run_in_ci == Some(true),
            "task '{name}' must have `options.runInCI: true` (got: {:?})",
            task.run_in_ci,
            name = task_name
        );
    }

    Ok(())
}
