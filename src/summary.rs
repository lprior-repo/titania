//! Human-readable summary output.

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct LaneResult {
    pub crate_name: String,
    pub lane: String,
    pub status: String,
    pub duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct RunSummary {
    pub run_id: String,
    pub results: Vec<LaneResult>,
}

impl RunSummary {
    pub fn pass_count(&self) -> usize {
        self.results.iter().filter(|r| r.status == "pass").count()
    }

    pub fn fail_count(&self) -> usize {
        self.results.iter().filter(|r| r.status == "fail").count()
    }

    pub fn skip_count(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status == "skip" || r.status == "dry-run")
            .count()
    }

    pub fn total_duration_ms(&self) -> u64 {
        self.results.iter().map(|r| r.duration_ms).sum()
    }

    pub fn has_failures(&self) -> bool {
        self.fail_count() > 0
    }
}

pub fn format_summary(summary: &RunSummary, json: bool) -> String {
    if json {
        format_json_summary(summary)
    } else {
        format_text_summary(summary)
    }
}

fn format_text_summary(summary: &RunSummary) -> String {
    let mut lines = Vec::new();
    lines.push(format!("=== xtask proof run: {} ===", summary.run_id));
    lines.push(String::new());

    let by_crate = group_by_crate(&summary.results);
    for (crate_name, results) in &by_crate {
        lines.push(format!("[{crate_name}]"));
        for r in results {
            let icon = status_icon(&r.status);
            lines.push(format!("  {} {} ({}ms)", icon, r.lane, r.duration_ms));
        }
        lines.push(String::new());
    }

    lines.push(format!(
        "Pass: {} | Fail: {} | Skip: {} | Total: {}ms",
        summary.pass_count(),
        summary.fail_count(),
        summary.skip_count(),
        summary.total_duration_ms()
    ));

    lines.join("\n")
}

fn format_json_summary(summary: &RunSummary) -> String {
    let output: HashMap<&str, serde_json::Value> = [
        ("run_id", serde_json::Value::String(summary.run_id.clone())),
        (
            "pass",
            serde_json::Value::Number(summary.pass_count().into()),
        ),
        (
            "fail",
            serde_json::Value::Number(summary.fail_count().into()),
        ),
        (
            "skip",
            serde_json::Value::Number(summary.skip_count().into()),
        ),
        (
            "total_ms",
            serde_json::Value::Number(summary.total_duration_ms().into()),
        ),
    ]
    .into_iter()
    .collect();

    serde_json::to_string_pretty(&output).unwrap_or_default()
}

fn group_by_crate(results: &[LaneResult]) -> HashMap<String, Vec<&LaneResult>> {
    let mut map: HashMap<String, Vec<&LaneResult>> = HashMap::new();
    for r in results {
        map.entry(r.crate_name.clone()).or_default().push(r);
    }
    map
}

fn status_icon(status: &str) -> &'static str {
    match status {
        "pass" => "✓",
        "fail" => "✗",
        "timeout" => "⏱",
        "skip" => "○",
        "dry-run" => "→",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(crate_name: &str, lane: &str, status: &str) -> LaneResult {
        LaneResult {
            crate_name: crate_name.to_string(),
            lane: lane.to_string(),
            status: status.to_string(),
            duration_ms: 100,
        }
    }

    #[test]
    fn test_pass_count() {
        let summary = RunSummary {
            run_id: "test".to_string(),
            results: vec![
                make_result("a", "test", "pass"),
                make_result("a", "clippy", "pass"),
                make_result("b", "test", "fail"),
            ],
        };
        assert_eq!(summary.pass_count(), 2);
        assert_eq!(summary.fail_count(), 1);
    }

    #[test]
    fn test_has_failures() {
        let summary = RunSummary {
            run_id: "test".to_string(),
            results: vec![make_result("a", "test", "pass")],
        };
        assert!(!summary.has_failures());

        let summary = RunSummary {
            run_id: "test".to_string(),
            results: vec![make_result("a", "test", "fail")],
        };
        assert!(summary.has_failures());
    }

    #[test]
    fn test_format_text_summary() {
        let summary = RunSummary {
            run_id: "run1".to_string(),
            results: vec![make_result("vb_core", "test", "pass")],
        };
        let text = format_text_summary(&summary);
        assert!(text.contains("vb_core"));
        assert!(text.contains("Pass:"));
    }
}
