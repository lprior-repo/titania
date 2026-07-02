use std::{
    fs,
    path::{Path, PathBuf},
};

/// One source line surfaced by [`walk_rs_lines`]: the line text, its
/// repository-relative path, and its 1-indexed line number.
pub(crate) struct WalkLine {
    pub(crate) text: String,
    pub(crate) path: String,
    pub(crate) line_no: u32,
}

/// Walk every `.rs` file beneath `root` and return one [`WalkLine`] per
/// source line, with paths expressed relative to `display_root`.
#[must_use]
pub(crate) fn walk_rs_lines(root: &Path, display_root: &Path) -> Vec<WalkLine> {
    let mut out = Vec::new();
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        visit_dir(&dir, display_root, &mut stack, &mut out);
    }
    out
}

fn visit_dir(dir: &Path, display_root: &Path, stack: &mut Vec<PathBuf>, out: &mut Vec<WalkLine>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    entries.flatten().for_each(|entry| visit_entry(entry.path(), display_root, stack, out));
}

fn visit_entry(
    path: PathBuf,
    display_root: &Path,
    stack: &mut Vec<PathBuf>,
    out: &mut Vec<WalkLine>,
) {
    if path.is_dir() {
        stack.push(path);
        return;
    }
    if path.extension().and_then(|s| s.to_str()) == Some("rs") {
        visit_file(&path, display_root, out);
    }
}

fn visit_file(path: &Path, display_root: &Path, out: &mut Vec<WalkLine>) {
    let Ok(text) = fs::read_to_string(path) else { return };
    let rel_path = path.strip_prefix(display_root).map_or(path, std::convert::identity);
    let rel = rel_path.to_string_lossy().into_owned();
    let mut line_no = 0_u32;
    text.lines().for_each(|line| {
        line_no = line_no.saturating_add(1);
        out.push(WalkLine { text: line.to_owned(), path: rel.clone(), line_no });
    });
}
