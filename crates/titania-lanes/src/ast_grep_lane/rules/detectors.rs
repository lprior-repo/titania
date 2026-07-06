//! String-backed detectors for rules ast-grep cannot express.
//!
//! Real ast-grep powers the structural FUNC_* rules (see [`super::super::engine`]).
//! Two rule families stay string-backed here:
//!
//! * `BYPASS_INLINE_SUPPRESSION` — comments are not AST nodes, so ast-grep
//!   cannot see `// ast-grep-ignore` / `// sg-ignore`.
//! * `ARCHITECTURE_IMPORT_CORE_*` — the rule couples a path-scope filter
//!   (file under `core/` / `domain/`) with import-text detection that
//!   covers grouped imports, multiline `use` blocks, and boundary-name
//!   exclusions (`fs_extra` is not `fs`). Expressing all of that as one
//!   ast-grep pattern requires a path-aware selector the engine does not
//!   own; the hand-rolled detector stays until a path-aware port lands.

mod code_scan;

use code_scan::{code_only_source, detect_code_line};

pub(super) fn detect_inline_suppression(source: &str) -> bool {
    source.lines().any(line_comment_contains_inline_suppression)
}

fn line_comment_contains_inline_suppression(line: &str) -> bool {
    line.split_once("//")
        .map(|(_, comment)| comment)
        .is_some_and(contains_inline_suppression_marker)
        || line
            .split_once("/*")
            .map(|(_, comment)| comment)
            .is_some_and(contains_inline_suppression_marker)
}

fn contains_inline_suppression_marker(comment: &str) -> bool {
    comment.contains("ast-grep-ignore") || comment.contains("sg-ignore")
}

pub(super) fn detect_core_infra_import(source: &str) -> bool {
    detect_code_line(source, |line| {
        line.contains("use tokio::")
            || line.contains("use axum::")
            || line.contains("use sqlx::")
            || line.contains("use reqwest::")
    })
}

pub(super) fn detect_core_fs_import(source: &str) -> bool {
    direct_import_contains(source, "std::", &["fs", "env", "net"])
        || grouped_std_import_contains(source, &["fs", "env", "net"])
}

pub(super) fn detect_core_time_import(source: &str) -> bool {
    direct_import_contains(source, "std::time::", &["SystemTime", "Instant"])
        || grouped_import_contains(source, "use std::time::", &["SystemTime", "Instant"])
}

pub(super) fn detect_core_random_import(source: &str) -> bool {
    direct_import_contains(source, "rand::", &["thread_rng", "Rng"])
        || grouped_rand_import_contains(source, &["thread_rng", "Rng"])
}

fn direct_import_contains(source: &str, path_prefix: &str, names: &[&str]) -> bool {
    let code = code_only_source(source);
    code.match_indices("use ")
        .filter_map(|(start, _)| direct_import_member(&code, start, path_prefix))
        .any(|member| names.iter().any(|name| grouped_member_matches(member, name)))
}

fn direct_import_member<'a>(source: &'a str, start: usize, path_prefix: &str) -> Option<&'a str> {
    let (_, from_use) = source.split_at_checked(start)?;
    use_item_prefix_allowed(source, start).then_some(())?;
    Some(from_use.strip_prefix("use ")?.strip_prefix(path_prefix)?.split_once(';')?.0.trim())
}

fn use_item_prefix_allowed(source: &str, start: usize) -> bool {
    source
        .split_at_checked(start)
        .map(|(before, _)| before)
        .map(current_line_prefix)
        .is_some_and(visibility_prefix_allows_use)
}

fn current_line_prefix(before_use: &str) -> &str {
    before_use.rsplit_once('\n').map_or(before_use, |(_, line)| line).trim_start()
}

fn visibility_prefix_allows_use(prefix: &str) -> bool {
    let prefix = prefix.trim_end();
    prefix.is_empty()
        || prefix == "pub"
        || prefix.strip_prefix("pub(").is_some_and(|rest| rest.ends_with(')'))
}

fn grouped_std_import_contains(source: &str, names: &[&str]) -> bool {
    grouped_import_contains(source, "use std::", names)
}

fn grouped_rand_import_contains(source: &str, names: &[&str]) -> bool {
    grouped_import_contains(source, "use rand::", names)
}

fn grouped_import_contains(source: &str, prefix: &str, names: &[&str]) -> bool {
    let code = code_only_source(source);
    code.match_indices(prefix)
        .filter_map(|(start, _)| grouped_import_body(&code, start, prefix))
        .any(|body| names.iter().any(|name| grouped_body_contains_name(body, name)))
}

fn grouped_import_body<'a>(source: &'a str, start: usize, prefix: &str) -> Option<&'a str> {
    let (_, from_prefix) = source.split_at_checked(start)?;
    use_item_prefix_allowed(source, start).then_some(())?;
    Some(from_prefix.strip_prefix(prefix)?.trim_start().strip_prefix('{')?.split_once('}')?.0)
}

fn grouped_body_contains_name(body: &str, name: &str) -> bool {
    body.split(',').map(str::trim).any(|member| grouped_member_matches(member, name))
}

fn grouped_member_matches(member: &str, name: &str) -> bool {
    member
        .strip_prefix(name)
        .is_some_and(|rest| rest.is_empty() || rest.starts_with("::") || rest.starts_with(" as "))
}

#[cfg(test)]
mod detector_tests {
    use super::detect_core_infra_import;

    #[test]
    fn detect_core_infra_import_catches_real_use() {
        assert!(detect_core_infra_import("use tokio::task;"));
        assert!(detect_core_infra_import("use axum::Router;"));
        assert!(detect_core_infra_import("use sqlx::Migrator;"));
        assert!(detect_core_infra_import("use reqwest::Client;"));
    }

    #[test]
    fn detect_core_infra_import_skips_comment_mentions() {
        assert!(!detect_core_infra_import("// use tokio::task"));
        assert!(!detect_core_infra_import("/* use axum::Router */"));
        assert!(!detect_core_infra_import("#[doc = \"use sqlx::Migrator\"]"));
        assert!(!detect_core_infra_import("/// Requires `use reqwest::Client;`"));
    }

    #[test]
    fn detect_core_infra_import_skips_string_literal_mentions() {
        assert!(!detect_core_infra_import(r#""use tokio::task;""#));
        assert!(!detect_core_infra_import(r#""use axum::Router""#));
    }

    #[test]
    fn detect_core_infra_import_allows_clean_code() {
        let clean = r#"
use std::collections::HashMap;

pub fn compute(x: i32) -> i32 {
    x + 1
}
"#;
        assert!(!detect_core_infra_import(clean));
    }
}
