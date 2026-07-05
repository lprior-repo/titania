use std::path::Path;

use titania_lanes::{Finding, LaneReport, RuleId};

use super::identifiers::{candidate_tokens, extract_id_extern};

type SourceRange<'a> = (&'a str, usize, usize);

#[derive(Debug)]
struct ExternEntry {
    named: String,
    line_no: u32,
    path: String,
    start: usize,
    end: usize,
}

#[derive(Default)]
struct LedgerState {
    pending_id: Option<String>,
    pending_line: u32,
}

pub(crate) fn per_extern_pass(root: &Path, rule: &RuleId, report: &mut LaneReport) {
    let verif_dir = root.join("verification/verus");
    let Ok(read) = std::fs::read_dir(&verif_dir) else {
        return;
    };
    read.filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| is_extern_ledger(path))
        .fold((), |(), path| scan_extern_ledger(root, &path, rule, report));
}

fn scan_extern_ledger(root: &Path, path: &Path, rule: &RuleId, report: &mut LaneReport) {
    let rel = rel_path(root, path);
    let Ok(text) = std::fs::read_to_string(path) else {
        return;
    };
    parse_extern_ledger(&text)
        .into_iter()
        .fold((), |(), entry| check_entry(root, &rel, &entry, rule, report));
}

fn check_entry(
    root: &Path,
    rel: &str,
    entry: &ExternEntry,
    rule: &RuleId,
    report: &mut LaneReport,
) {
    let Ok(prod_text) = std::fs::read_to_string(root.join(&entry.path)) else {
        report.push(Finding::new(
            rule.clone(),
            rel.to_owned(),
            entry.line_no,
            format!("production source missing: {}", entry.path),
        ));
        return;
    };
    if !entry_found(&prod_text, entry) {
        report_missing_identifier(rel, entry, rule, report);
    }
}

fn entry_found(prod_text: &str, entry: &ExternEntry) -> bool {
    let lines: Vec<&str> = prod_text.lines().collect();
    let total = lines.len();
    let ctx_start = entry.start.saturating_sub(6);
    let ctx_end = entry.end.saturating_add(5).min(total);
    let window = lines.get(ctx_start..ctx_end).map_or_else(String::new, |lines| lines.join("\n"));
    let window_ids = extract_id_extern(&window);
    candidate_tokens(&entry.named).iter().any(|candidate| window_ids.contains(candidate))
}

fn report_missing_identifier(
    rel: &str,
    entry: &ExternEntry,
    rule: &RuleId,
    report: &mut LaneReport,
) {
    let candidates = candidate_tokens(&entry.named);
    report.push(Finding::new(
        rule.clone(),
        rel.to_owned(),
        entry.line_no,
        format!(
            "identifier {} (key {}) not found in {}:{}-{}",
            entry.named,
            candidates.join(","),
            entry.path,
            entry.start,
            entry.end
        ),
    ));
}

fn parse_extern_ledger(text: &str) -> Vec<ExternEntry> {
    text.lines()
        .enumerate()
        .fold((LedgerState::default(), Vec::new()), |(mut state, mut out), (idx, raw)| {
            let line_no = u32::try_from(idx).map_or(0, |value| value).saturating_add(1);
            parse_ledger_line(raw.trim_start(), line_no, &mut state, &mut out);
            (state, out)
        })
        .1
}

fn parse_ledger_line(
    line: &str,
    line_no: u32,
    state: &mut LedgerState,
    out: &mut Vec<ExternEntry>,
) {
    if let Some(id) = extract_bullet_id(line) {
        state.pending_id = Some(id);
        state.pending_line = line_no;
    }
    let Some((path, start, end)) = extract_arrow(line) else {
        return;
    };
    if !path.starts_with("crates/") {
        state.pending_id = None;
        return;
    }
    if line_no.saturating_sub(state.pending_line) > 5 {
        return;
    }
    if let Some(id) = state.pending_id.take() {
        out.push(ExternEntry { named: id, line_no, path: path.to_string(), start, end });
    }
}

fn extract_bullet_id(line: &str) -> Option<String> {
    let rest = line.strip_prefix("//")?.trim_start().strip_prefix('-')?.trim_start();
    let backtick = rest.find('`')?;
    let after = rest.get(backtick.saturating_add(1)..)?;
    let end = after.find('`')?;
    after.get(..end).map(str::to_string)
}

fn extract_arrow(line: &str) -> Option<SourceRange<'_>> {
    let pos = line.find("<-")?;
    let rest = line.get(pos.saturating_add(2)..)?.trim_start();
    let (path, range_str) = rest
        .split_once(':')
        .map_or_else(|| (rest.trim(), ""), |(path, range)| (path.trim(), range.trim()));
    let mut rparts = range_str.splitn(2, '-');
    let start = rparts.next().and_then(|s| s.parse().ok()).map_or(1, |value: usize| value);
    let end = rparts.next().and_then(|s| s.parse().ok()).map_or(start, |value: usize| value);
    Some((path, start, end))
}

fn rel_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root).map_or_else(
        |_| path.to_string_lossy().into_owned(),
        |rel| rel.to_string_lossy().replace('\\', "/"),
    )
}

fn is_extern_ledger(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()).is_some_and(|name| {
        name.starts_with("extern_")
            && std::path::Path::new(name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
    })
}
