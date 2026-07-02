fn collect_files(root: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    walk(root, &mut out);
    out.sort();
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for path in entries.filter_map(Result::ok).map(|entry| entry.path()) {
        visit_path(path, out);
    }
}

fn visit_path(path: PathBuf, out: &mut Vec<PathBuf>) {
    if path.is_dir() && !is_heavy_tree(&path) {
        walk(&path, out);
    } else if is_scanned_file(&path) {
        out.push(path);
    }
}

fn is_heavy_tree(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| matches!(name, "target" | "node_modules" | ".git"))
}

fn is_scanned_file(path: &Path) -> bool {
    path.extension().and_then(|e| e.to_str()).is_some_and(|ext| SCAN_EXTENSIONS.contains(&ext))
}

fn is_path_excluded(file: &Path) -> bool {
    let normalized = file.to_string_lossy().replace('\\', "/");
    contains_excluded_substring(&normalized) || has_excluded_filename(file)
}

fn contains_excluded_substring(normalized: &str) -> bool {
    EXCLUDED_SUBSTRINGS.iter().any(|s| normalized.contains(s))
}

fn has_excluded_filename(file: &Path) -> bool {
    let name = file.file_name().and_then(|n| n.to_str()).map_or("", core::convert::identity);
    name == "check-spelling-gate.sh"
        || name == "check_spelling_gate.rs"
        || name == "velvet-ballistics-MASTER.md"
        || name == "BIG-ASS-TESTING-TO-FIX.md"
        || name.ends_with("_tests.rs")
        || name.contains("final-")
        || name.contains("proof-repair-")
        || name.contains("black-hat-review-")
}
