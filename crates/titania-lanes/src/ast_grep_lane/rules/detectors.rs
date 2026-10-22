mod code_scan;

use code_scan::{code_only_source, detect_code_line};

pub(super) fn detect_core_infra_import(source: &str) -> bool {
    detect_code_line(source, |line| {
        line.contains("use tokio::")
            || line.contains("use axum::")
            || line.contains("use sqlx::")
            || line.contains("use reqwest::")
    }) || fully_qualified_uses_path(source, &["tokio::", "axum::", "sqlx::", "reqwest::"])
        || crate_alias_path_used(source, &["tokio", "axum", "sqlx", "reqwest"])
}

pub(super) fn detect_core_fs_import(source: &str) -> bool {
    direct_import_contains(source, "std::", &["fs", "env", "net"])
        || grouped_std_import_contains(source, &["fs", "env", "net"])
        || fully_qualified_uses_path(source, &["std::fs::", "std::env::", "std::net::"])
}

pub(super) fn detect_core_time_import(source: &str) -> bool {
    direct_import_contains(source, "std::time::", &["SystemTime", "Instant"])
        || grouped_import_contains(source, "use std::time::", &["SystemTime", "Instant"])
        || fully_qualified_uses_path(source, &["std::time::SystemTime", "std::time::Instant"])
}

pub(super) fn detect_core_random_import(source: &str) -> bool {
    direct_import_contains(source, "rand::", &["thread_rng"])
        || grouped_rand_import_contains(source, &["thread_rng"])
        || fully_qualified_uses_path(source, &["rand::thread_rng"])
}

pub(super) fn detect_core_process_import(source: &str) -> bool {
    direct_import_contains(source, "std::", &["process"])
        || grouped_std_import_contains(source, &["process"])
        || fully_qualified_uses_path(
            source,
            &[
                "std::process::",
                "tokio::process",
                "std::os::unix::process",
                "std::os::windows::process",
            ],
        )
}

pub(super) fn detect_core_async_effect(source: &str) -> bool {
    let code = code_only_source(source);
    code_contains_token(&code, "async fn")
        || code_contains_token(&code, "async unsafe fn")
        || contains_await_expression(&code)
        || fully_qualified_uses_path(source, &["tokio::spawn", "tokio::time"])
        || tokio_task_used(&code)
        || tokio_alias_effect_used(source)
}

fn contains_await_expression(code: &str) -> bool {
    code.match_indices(".await").any(|(start, _)| {
        code.split_at_checked(start.saturating_add(".await".len()))
            .is_none_or(|(_, after)| after.chars().next().is_none_or(|next| !is_ident_char(next)))
    })
}

fn tokio_task_used(code: &str) -> bool {
    code.lines().any(|line| code_contains_token(line, "tokio::task") && !is_bare_task_import(line))
}

fn is_bare_task_import(line: &str) -> bool {
    let statement = line.trim().trim_start_matches("pub ").trim_start();
    let statement = statement
        .strip_prefix("pub(")
        .and_then(|rest| rest.split_once(')'))
        .map_or(statement, |(_, tail)| tail.trim_start());
    statement == "use tokio::task;" || statement == "use tokio::task::{self};"
}

fn tokio_alias_effect_used(source: &str) -> bool {
    let code = code_only_source(source);
    crate_root_aliases(&code, "tokio").iter().any(|alias| {
        ["spawn", "task", "time"]
            .iter()
            .any(|member| code_contains_token(&code, &format!("{alias}::{member}")))
    })
}

pub(super) fn detect_core_state_effect(source: &str) -> bool {
    fully_qualified_uses_path(
        source,
        &[
            "static mut",
            "std::sync::Mutex",
            "std::sync::RwLock",
            "std::cell::RefCell",
            "std::thread::spawn",
        ],
    ) || direct_import_contains(source, "std::sync::", &["Mutex", "RwLock"])
        || grouped_import_contains(source, "use std::sync::", &["Mutex", "RwLock"])
        || direct_import_contains(source, "std::cell::", &["RefCell"])
        || grouped_import_contains(source, "use std::cell::", &["RefCell"])
        || grouped_import_contains(source, "use std::thread::", &["spawn"])
}

fn fully_qualified_uses_path(source: &str, needles: &[&str]) -> bool {
    let code = code_only_source(source);
    needles.iter().any(|needle| code_contains_token(&code, needle))
}

fn code_contains_token(code: &str, needle: &str) -> bool {
    code.match_indices(needle).any(|(start, _)| {
        leading_boundary_ok(code, start) && trailing_boundary_ok(code, start, needle)
    })
}

fn leading_boundary_ok(code: &str, start: usize) -> bool {
    code.split_at_checked(start).is_some_and(|(before, _)| {
        before.chars().next_back().is_none_or(|prev| !is_ident_char(prev))
    })
}

fn trailing_boundary_ok(code: &str, start: usize, needle: &str) -> bool {
    needle.ends_with("::")
        || code
            .split_at_checked(start.saturating_add(needle.len()))
            .is_none_or(|(_, after)| after.chars().next().is_none_or(|next| !is_ident_char(next)))
}

fn is_ident_char(ch: char) -> bool {
    ch.is_alphanumeric() || ch == '_'
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

fn crate_alias_path_used(source: &str, crates: &[&str]) -> bool {
    let code = code_only_source(source);
    crates.iter().any(|krate| {
        crate_root_aliases(&code, krate)
            .iter()
            .any(|alias| code_contains_token(&code, &format!("{alias}::")))
    })
}

pub(super) fn crate_root_aliases(code: &str, krate: &str) -> Vec<String> {
    code.match_indices("use ")
        .filter_map(|(start, _)| crate_root_alias_at(code, start, krate))
        .collect()
}

fn crate_root_alias_at(source: &str, start: usize, krate: &str) -> Option<String> {
    use_item_prefix_allowed(source, start).then_some(())?;
    let statement =
        source.split_at_checked(start)?.1.strip_prefix("use ")?.split_once(';')?.0.trim();
    statement.strip_prefix(krate).and_then(alias_from_use_rest)
}

fn alias_from_use_rest(rest: &str) -> Option<String> {
    if let Some(alias) = rest.trim().strip_prefix("as ") {
        return valid_alias(alias.trim());
    }
    let grouped = rest.trim().strip_prefix("::{")?.strip_suffix('}')?;
    grouped
        .split(',')
        .map(str::trim)
        .find_map(|member| member.strip_prefix("self as "))
        .and_then(|alias| valid_alias(alias.trim()))
}

fn valid_alias(alias: &str) -> Option<String> {
    (!alias.is_empty() && alias.chars().all(is_ident_char)).then(|| alias.to_owned())
}
