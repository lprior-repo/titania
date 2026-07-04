//! Multi-line `#![feature(...)]` attribute collection.
//!
//! The collector is an explicit two-state machine (`CollectorState`):
//! `Idle` (no attribute in progress) or `Collecting { first_line, buf }`
//! (accumulating a multi-line attribute body until `)]` closes it). The
//! `first_line` is only meaningful while collecting, which the enum
//! makes type-enforced — no `Option<String>` + separate `u32` field that
//! can disagree about whether a collection is open.

/// `(feature_line, names, first_occurrence_line)` tuple.
///
/// The first occurrence line is what the bash reports; subsequent lines in
/// a multi-line attribute produce no extra message. We surface one finding
/// per individual feature, attached to the first line of the attribute.
pub(super) type FeatureUse = (u32, Vec<String>, u32);

/// Two-state collector lifecycle.
enum CollectorState {
    Idle,
    Collecting { first_line: u32, buf: String },
}

/// Collect every `#![feature(...)]` attribute body in `content`,
/// including multi-line forms that span until `)]`.
pub(super) fn collect_features(content: &str) -> Vec<FeatureUse> {
    let mut out = Vec::new();
    let final_state =
        content.lines().enumerate().fold(CollectorState::Idle, |state, (idx, line)| {
            collect_feature_line(state, feature_line_no(idx), line.trim(), &mut out)
        });
    if let CollectorState::Collecting { buf, .. } = final_state {
        let _emitted = crate::write_stderr_line(format_args!(
            "unterminated unstable feature attribute starting with `{buf}`"
        ))
        .is_ok();
    }
    out
}

fn feature_line_no(idx: usize) -> u32 {
    u32::try_from(idx.saturating_add(1)).map_or(u32::MAX, |line_no| line_no)
}

fn collect_feature_line(
    state: CollectorState,
    line_no: u32,
    trimmed: &str,
    out: &mut Vec<FeatureUse>,
) -> CollectorState {
    match state {
        CollectorState::Idle => collect_idle_feature_line(line_no, trimmed, out),
        CollectorState::Collecting { first_line, buf } => {
            collect_accumulated_line(first_line, buf, trimmed, out)
        }
    }
}

fn collect_idle_feature_line(
    line_no: u32,
    trimmed: &str,
    out: &mut Vec<FeatureUse>,
) -> CollectorState {
    let Some(after_open) = trimmed.strip_prefix("#![feature(") else {
        return CollectorState::Idle;
    };
    collect_feature_start(line_no, after_open, out)
}

/// If we are mid-attribute, append the line and check for `)]`. Returns
/// `Some(next_state)` when collecting, `None` when idle.
fn collect_accumulated_line(
    first_line: u32,
    mut buf: String,
    trimmed: &str,
    out: &mut Vec<FeatureUse>,
) -> CollectorState {
    buf.push(' ');
    buf.push_str(trimmed);
    let _ = push_closed_feature(first_line, &buf, out);
    if buf.contains(")]") {
        CollectorState::Idle
    } else {
        CollectorState::Collecting { first_line, buf }
    }
}

fn collect_feature_start(
    line_no: u32,
    after_open: &str,
    out: &mut Vec<FeatureUse>,
) -> CollectorState {
    if push_closed_feature(line_no, after_open, out) {
        return CollectorState::Idle;
    }
    CollectorState::Collecting { first_line: line_no, buf: format!("#![feature({after_open}") }
}

fn push_closed_feature(line_no: u32, text: &str, out: &mut Vec<FeatureUse>) -> bool {
    let Some(close_idx) = text.find(")]") else {
        return false;
    };
    // `close_idx` points at the `)`; the matching `]` is the next byte.
    // Include both so `extract_names` can strip the `)]` suffix without
    // leaving a stray `)` on the last feature name.
    let end = close_idx.saturating_add(2);
    let Some(slice) = text.get(..end) else {
        return true;
    };
    if slice.ends_with(")]") {
        out.push((line_no, extract_names(slice), line_no));
    }
    true
}

fn extract_names(inside: &str) -> Vec<String> {
    // `inside` starts with `#![feature(` and ends with `)]`. Strip
    // those and split on commas.
    let body = inside.trim_start_matches("#![feature(").trim_end_matches(")]");
    body.split(',').map(|s| s.trim().to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::{FeatureUse, collect_features, push_closed_feature};

    #[test]
    fn push_closed_feature_extracts_single_line_attribute_without_stray_paren() {
        // Regression: the slice used to end at the `)` of `)]`,
        // leaving the `]` out. `extract_names` then failed to strip
        // the `)]` suffix and the last feature name carried a stray
        // `)`.
        let mut out: Vec<FeatureUse> = Vec::new();
        assert!(push_closed_feature(1, "try_blocks)]", &mut out));
        assert_eq!(out.len(), 1);
        let (line_no, names, _) = &out[0];
        assert_eq!(*line_no, 1);
        assert_eq!(names, &vec!["try_blocks".to_owned()]);
    }

    #[test]
    fn push_closed_feature_extracts_multi_feature_attribute() {
        let mut out: Vec<FeatureUse> = Vec::new();
        assert!(push_closed_feature(1, "allocator_api, generic_const_exprs)]", &mut out));
        let names = &out[0].1;
        assert_eq!(names, &vec!["allocator_api".to_owned(), "generic_const_exprs".to_owned()]);
    }

    #[test]
    fn push_closed_feature_returns_false_when_no_close() {
        let mut out: Vec<FeatureUse> = Vec::new();
        assert!(!push_closed_feature(1, "try_blocks", &mut out));
        assert!(out.is_empty());
    }

    #[test]
    fn collect_features_handles_multi_line_attribute() {
        let content = "#![feature(\n    allocator_api,\n    generic_const_exprs\n)]\n";
        let uses = collect_features(content);
        assert_eq!(uses.len(), 1);
        let (_line, names, _report) = &uses[0];
        assert_eq!(names, &vec!["allocator_api".to_owned(), "generic_const_exprs".to_owned()]);
    }

    #[test]
    fn collect_features_handles_single_line_attribute() {
        let content = "#![feature(try_blocks)]\n";
        let uses = collect_features(content);
        assert_eq!(uses.len(), 1);
        let (_line, names, _report) = &uses[0];
        assert_eq!(names, &vec!["try_blocks".to_owned()]);
    }

    #[test]
    fn collect_features_ignores_non_attribute_text() {
        // The `#![feature(` prefix is required; this line just
        // mentions `feature` in a comment.
        let content = "// #![feature(specialization)]\nlet x = 1;\n";
        let uses = collect_features(content);
        assert!(uses.is_empty());
    }
}
