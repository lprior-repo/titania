//! Integration test for `.moon/tasks/all.yml` — v1 §13 task DAG.
//!
//! Asserts the exact v1-spec contract for all 13 required tasks (10 lane +
//! 3 gate composites) co-exist with legacy cargo-native tasks. The test
//! intentionally fails RED when v1 tasks are absent, proving the parser and
//! assertion logic work (not a silent pass).
//!
//! Key contracts from §13:
//!   • deps use `~:name` (Moon local task dep syntax)
//!   • gate tasks have NO toolchains key
//!   • compile fan-out: clippy, test, build depend on `~:titania-compile`
//!   • env/inputs/outputs declared per §13 YAML

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
                deps = parse_flow_deps(val.trim());
            }
            // `deps:` block list (indented `- '...'` lines)
            else if trimmed == "deps:" {
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
            inputs,
            outputs,
            run_in_ci,
        });
    }

    tasks
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

/// All 13 required v1 tasks from §13 (10 lane + 3 gate composites).
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
    "gate-edit",
    "gate-prepush",
    "gate-release",
];

const LANE_COMMANDS: &[(&str, &str)] = &[
    ("titania-fmt", "cargo run --quiet -p titania-check -- run-lane fmt"),
    (
        "titania-compile",
        "CARGO_TARGET_DIR=\".titania/cache/compile\" cargo run --quiet -p titania-check -- run-lane compile",
    ),
    (
        "titania-clippy",
        "CARGO_TARGET_DIR=\".titania/cache/clippy\" cargo run --quiet -p titania-check -- run-lane clippy",
    ),
    ("titania-ast-grep", "cargo run --quiet -p titania-check -- run-lane ast-grep"),
    (
        "titania-dylint",
        "PATH=\"$HOME/.local/share/mise/shims:$HOME/.cargo/bin:$PATH\" cargo run --quiet -p titania-check -- run-lane dylint",
    ),
    ("titania-panic-scan", "cargo run --quiet -p titania-check -- run-lane panic-scan"),
    ("titania-policy-scan", "cargo run --quiet -p titania-check -- run-lane policy-scan"),
    (
        "titania-test",
        "PATH=\"$HOME/.local/share/mise/shims:$HOME/.cargo/bin:$PATH\" CARGO_TARGET_DIR=\".titania/cache/test\" cargo run --quiet -p titania-check -- run-lane test",
    ),
    ("titania-deny", "cargo run --quiet -p titania-check -- run-lane deny"),
    (
        "titania-build",
        "CARGO_TARGET_DIR=\".titania/cache/release\" cargo run --quiet -p titania-check -- run-lane build",
    ),
];

/// Expected commands for each gate composite.
const GATE_COMMANDS: &[(&str, &str)] =
    &[("gate-edit", "edit"), ("gate-prepush", "prepush"), ("gate-release", "release")];

/// Lane tasks that MUST have `toolchains: [rust]` (§13).
const RUST_TOOLCHAIN_TASKS: &[&str] = &[
    "titania-fmt",
    "titania-compile",
    "titania-clippy",
    "titania-dylint",
    "titania-test",
    "titania-build",
];

/// Lane tasks that MUST NOT have a `toolchains` key (§13).
const NO_TOOLCHAIN_TASKS: &[&str] =
    &["titania-ast-grep", "titania-panic-scan", "titania-policy-scan", "titania-deny"];

/// Gate tasks that MUST NOT have a `toolchains` key (§13).
const GATE_NO_TOOLCHAIN_TASKS: &[&str] = &["gate-edit", "gate-prepush", "gate-release"];

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
            "Cargo.toml",
            "**/Cargo.toml",
            ".cargo/**",
            ".titania/**",
            "!.titania/cache/**",
            "!.titania/out/**",
        ],
    ),
    ("titania-test", &["@globs(sources)", "@globs(tests)"]),
    ("titania-deny", &["Cargo.lock", "deny.toml"]),
    ("titania-build", &["@globs(sources)"]),
];

/// Expected outputs per v1 lane task (§13).
const LANE_OUTPUTS: &[(&str, &[&str])] =
    &[("titania-build", &["target/release/titania-check", ".titania/out/release/build.json"])];

/// Lane tasks keep cache/PATH exports inline so Moon shell expansion, not YAML
/// env interpolation, controls target directories and rustup availability.
const LANE_EMPTY_ENV: &[&str] =
    &["titania-compile", "titania-clippy", "titania-dylint", "titania-test", "titania-build"];

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

    // ── 5. Assert gate commands ────────────────────────────────────────────
    for (gate_name, scope) in GATE_COMMANDS {
        let task = task_map
            .get(*gate_name)
            .with_context(|| format!("gate task '{gate_name}' not found in parsed tasks"))?;
        let expected_cmd =
            format!("cargo run --quiet -p titania-check -- aggregate --scope {scope}");
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

    // ── 12. Assert ALL 13 v1 tasks have a command ─────────────────────────
    for task_name in REQUIRED_TASKS {
        let task =
            task_map.get(*task_name).with_context(|| format!("task '{task_name}' not found"))?;
        assert!(
            task.command.is_some(),
            "task '{task_name}' must have a `command:` field (got: {:?})",
            task.command
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

    // ── 16. Assert runInCI: true for all 13 v1 tasks ──────────────────────
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
