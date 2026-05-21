#![forbid(unsafe_code)]

use anyhow::Context;

// `forbidden-scan` xtask command.
// Scans first-party Rust crates for forbidden unsafe patterns and discipline violations.
// Returns non-zero exit code if any forbidden patterns are found.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Default first-party crates to scan when no globs are provided.
const DEFAULT_FIRST_PARTY_CRATES: &[&str] = &[
    "vb_benchmark",
    "vb_codegen",
    "vb_compile",
    "vb_core",
    "vb_doc",
    "vb_expr",
    "vb_ipc",
    "vb_proof_kernels",
    "vb_runtime",
    "vb_storage",
    "vb_ui",
    "vb_ui_makepad",
    "vb_ui_model",
    "vb_ui_snapshot",
    "vb_validate",
    "vb_yaml",
    "vb_cli",
    "workspace_tests",
];

/// Forbidden patterns: (pattern, description)
const FORBIDDEN_PATTERNS: &[(&str, &str)] = &[
    (r"Arc::unwrap\b", "Arc::unwrap call"),
    (r"Arc::expect\b", "Arc::expect call"),
    (r"\b\.unwrap\(\)", "arbitrary .unwrap() call"),
    (r"\b\.expect\(", "arbitrary .expect() call"),
    (r"\bpanic!\s*\(", "panic! macro invocation"),
    (r"\btodo!\s*\(", "todo! macro invocation"),
    (r"\bunimplemented!\s*\(", "unimplemented! macro invocation"),
];

/// Unsafe block pattern (to detect unsafe { } blocks).
const UNSAFE_BLOCK_PATTERN: &str = r"\bunsafe\s*\{";

#[derive(Debug)]
pub struct ScanResult {
    pub pattern: String,
    pub description: String,
    pub file: PathBuf,
    pub line: usize,
    pub line_content: String,
}

#[derive(Debug)]
pub struct ScanSummary {
    pub total_findings: usize,
    pub findings_by_pattern: std::collections::HashMap<String, usize>,
    pub files_with_findings: usize,
    pub crates_scanned: Vec<String>,
}

impl ScanSummary {
    pub fn new() -> Self {
        Self {
            total_findings: 0,
            findings_by_pattern: std::collections::HashMap::new(),
            files_with_findings: 0,
            crates_scanned: Vec::new(),
        }
    }

    pub fn add_finding(&mut self, pattern: &str) {
        self.total_findings = self.total_findings.saturating_add(1);
        let count = self
            .findings_by_pattern
            .entry(pattern.to_string())
            .or_insert(0);
        *count = count.saturating_add(1);
    }
}

/// Run the forbidden-scan command.
pub fn cmd_forbidden_scan(
    crate_globs: Option<&[String]>,
    allowlist_path: Option<&str>,
) -> anyhow::Result<()> {
    let workspace_root =
        std::env::current_dir().context("Failed to determine current working directory")?;

    let crates_to_scan = resolve_crates(&workspace_root, crate_globs)?;
    let allowlist = load_allowlist(allowlist_path)?;

    let mut all_findings: Vec<ScanResult> = Vec::new();
    let mut summary = ScanSummary::new();

    for crate_name in &crates_to_scan {
        let crate_path = workspace_root.join("crates").join(crate_name);
        let src_path = crate_path.join("src");

        if !src_path.exists() {
            // Try root src for single-crate workspaces
            let root_src = crate_path.join("src");
            if !root_src.exists() {
                write_stderr_line(format_args!(
                    "warning: no src/ found for crate {}, skipping",
                    crate_name
                ))?;
                continue;
            }
        }

        let scan_root = if src_path.exists() {
            &src_path
        } else {
            &crate_path
        };

        write_stderr_line(format_args!("Scanning crate: {}", crate_name))?;

        let crate_findings = scan_crate(scan_root, &allowlist)?;
        for finding in &crate_findings {
            summary.add_finding(&finding.pattern);
        }
        all_findings.extend(crate_findings);
        summary.crates_scanned.push(crate_name.clone());
    }

    // Deduplicate findings by file:line:pattern
    all_findings.sort_by(|a, b| (&a.file, a.line, &a.pattern).cmp(&(&b.file, b.line, &b.pattern)));
    all_findings.dedup_by(|a, b| a.file == b.file && a.line == b.line && a.pattern == b.pattern);

    summary.files_with_findings = all_findings
        .iter()
        .map(|f| f.file.clone())
        .collect::<std::collections::HashSet<_>>()
        .len();

    // Report findings
    if all_findings.is_empty() {
        write_stdout_line(format_args!(
            "forbidden-scan: PASS — no forbidden patterns found"
        ))?;
        write_stdout_line(format_args!(""))?;
        write_stdout_line(format_args!(
            "Crates scanned: {}",
            summary.crates_scanned.join(", ")
        ))?;
        return Ok(());
    }

    write_stdout_line(format_args!(
        "forbidden-scan: FAIL — {} forbidden pattern(s) found",
        all_findings.len()
    ))?;
    write_stdout_line(format_args!(""))?;
    write_stdout_line(format_args!("Summary:"))?;
    write_stdout_line(format_args!("  Total findings: {}", summary.total_findings))?;
    write_stdout_line(format_args!(
        "  Files with findings: {}",
        summary.files_with_findings
    ))?;
    write_stdout_line(format_args!(
        "  Crates scanned: {}",
        summary.crates_scanned.join(", ")
    ))?;
    write_stdout_line(format_args!(""))?;
    write_stdout_line(format_args!("Findings by pattern:"))?;
    for (pattern, count) in &summary.findings_by_pattern {
        write_stdout_line(format_args!("  {}: {}", pattern, count))?;
    }
    write_stdout_line(format_args!(""))?;

    // Print detailed findings
    let mut current_file: Option<PathBuf> = None;
    for finding in &all_findings {
        if current_file.as_ref() != Some(&finding.file) {
            write_stdout_line(format_args!("{}:", finding.file.display()))?;
            current_file = Some(finding.file.clone());
        }
        write_stdout_line(format_args!(
            "  {}:{}: {} [{}]",
            finding.line,
            finding.line_content.trim(),
            finding.pattern,
            finding.description
        ))?;
    }

    anyhow::bail!("forbidden patterns detected");
}

fn resolve_crates(
    _workspace_root: &PathBuf,
    globs: Option<&[String]>,
) -> anyhow::Result<Vec<String>> {
    match globs {
        Some(globs) if !globs.is_empty() => {
            let mut crates = Vec::new();
            for glob in globs {
                let matched: Vec<String> = DEFAULT_FIRST_PARTY_CRATES
                    .iter()
                    .filter(|c| glob_match(glob, c))
                    .map(|s| s.to_string())
                    .collect();
                crates.extend(matched);
            }
            crates.sort();
            crates.dedup();
            Ok(crates)
        }
        _ => Ok(DEFAULT_FIRST_PARTY_CRATES
            .iter()
            .map(|s| (*s).to_string())
            .collect()),
    }
}

fn glob_match(pattern: &str, name: &str) -> bool {
    if pattern == name {
        return true;
    }
    // Simple glob: treat * as wildcard
    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        match parts.as_slice() {
            [prefix, suffix] => return name.starts_with(prefix) && name.ends_with(suffix),
            [prefix] => return name.starts_with(prefix),
            _ => {}
        }
    }
    false
}

fn load_allowlist(path: Option<&str>) -> anyhow::Result<Vec<String>> {
    match path {
        Some(p) => {
            let content = fs::read_to_string(p)
                .with_context(|| format!("Failed to read allowlist file: {}", p))?;
            Ok(content
                .lines()
                .map(str::trim)
                .filter(|l| !l.is_empty() && !l.starts_with('#'))
                .map(String::from)
                .collect())
        }
        None => Ok(Vec::new()),
    }
}

fn scan_crate(crate_src_path: &Path, allowlist: &[String]) -> anyhow::Result<Vec<ScanResult>> {
    let mut findings = Vec::new();

    // Find all .rs files
    let rs_files = collect_rs_files(crate_src_path)?;

    for file_path in &rs_files {
        // Check if file is in generated/perf directory or is a test file (skip those)
        let rel_path = file_path.strip_prefix(crate_src_path).unwrap_or(file_path);
        let rel_str = rel_path.to_string_lossy();
        if rel_str.contains("generated")
            || rel_str.contains("perf")
            || rel_str.contains("target")
            || rel_str.starts_with("tests/")
            || rel_str.contains("_tests.")
            || rel_str.ends_with("_tests.rs")
            || rel_str.ends_with("tests.rs")
            || rel_str.contains("/test")
            || rel_str.starts_with("kani/")
            || rel_str.contains("/kani/")
        {
            continue;
        }

        let file_findings = scan_file(file_path, allowlist)?;
        findings.extend(file_findings);
    }

    Ok(findings)
}

fn collect_rs_files(dir: &Path) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    if !dir.is_dir() {
        return Ok(files);
    }

    let mut stack = vec![dir.to_path_buf()];
    while let Some(current) = stack.pop() {
        let entries = fs::read_dir(&current)
            .with_context(|| format!("Failed to read directory: {}", current.display()))?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir() {
                // Skip test dirs and target dirs
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy())
                    .unwrap_or_default();
                if name != "target" && name != "tests" && !name.starts_with('.') {
                    stack.push(path);
                }
            } else if path.extension().map(|e| e == "rs").unwrap_or(false) {
                files.push(path);
            }
        }
    }
    Ok(files)
}

fn scan_file(file_path: &PathBuf, allowlist: &[String]) -> anyhow::Result<Vec<ScanResult>> {
    let content = fs::read_to_string(file_path)
        .with_context(|| format!("Failed to read file: {}", file_path.display()))?;

    let mut findings = Vec::new();
    let lines: Vec<&str> = content.lines().collect();

    // Check forbidden patterns
    for (pattern, description) in FORBIDDEN_PATTERNS {
        // Skip if pattern is in allowlist
        if allowlist.iter().any(|a| a == *pattern) {
            continue;
        }

        for (line_num, line) in lines.iter().enumerate() {
            if line.contains("dbg!") && pattern == &r"\b\.unwrap\(\)" {
                // Skip dbg! invocations that might contain .unwrap()
                continue;
            }
            if regex_contains(line, pattern) {
                findings.push(ScanResult {
                    pattern: pattern.to_string(),
                    description: description.to_string(),
                    file: file_path.clone(),
                    line: checked_line_number(line_num),
                    line_content: line.to_string(),
                });
            }
        }
    }

    // Check unsafe blocks (excluding allowlisted)
    for (line_num, line) in lines.iter().enumerate() {
        if regex_contains(line, UNSAFE_BLOCK_PATTERN) {
            let is_allowlisted = allowlist.iter().any(|a| regex_contains(line, a));
            if !is_allowlisted {
                findings.push(ScanResult {
                    pattern: UNSAFE_BLOCK_PATTERN.to_string(),
                    description: "unsafe { } block (not allowlisted)".to_string(),
                    file: file_path.clone(),
                    line: checked_line_number(line_num),
                    line_content: line.to_string(),
                });
            }
        }
    }

    Ok(findings)
}

fn checked_line_number(line_index: usize) -> usize {
    match line_index.checked_add(1) {
        Some(line) => line,
        None => usize::MAX,
    }
}

fn write_stdout_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_fmt(args)?;
    handle.write_all(b"\n")
}

fn write_stderr_line(args: std::fmt::Arguments<'_>) -> io::Result<()> {
    let stderr = io::stderr();
    let mut handle = stderr.lock();
    handle.write_fmt(args)?;
    handle.write_all(b"\n")
}

fn regex_contains(haystack: &str, pattern: &str) -> bool {
    // Use simple substring matching for known patterns to avoid regex crate dependency
    // For complex patterns, fall back to regex crate
    match pattern {
        r"Arc::unwrap\b" => haystack.contains("Arc::unwrap"),
        r"Arc::expect\b" => haystack.contains("Arc::expect"),
        r"\b\.unwrap\(\)" => {
            // Match .unwrap() but not dbg!.unwrap()
            haystack.contains(".unwrap()") && !haystack.contains("dbg!.unwrap()")
        }
        r"\b\.expect\(" => haystack.contains(".expect(") && !haystack.contains("dbg!.expect("),
        r"\bpanic!\s*\(" => haystack.contains("panic!"),
        r"\btodo!\s*\(" => haystack.contains("todo!"),
        r"\bunimplemented!\s*\(" => haystack.contains("unimplemented!"),
        r"\bunsafe\s*\{" => haystack.contains("unsafe {"),
        _ => {
            // Fall back to regex for complex patterns
            use regex::Regex;
            let re = match Regex::new(pattern) {
                Ok(r) => r,
                Err(_) => return haystack.contains(pattern),
            };
            re.is_match(haystack)
        }
    }
}
