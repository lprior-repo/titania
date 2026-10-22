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
        || grouped_std_import_contains(source, &["time::SystemTime", "time::Instant"])
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
    let statements = use_statement_ranges(code);
    code.match_indices("tokio::task").any(|(start, _)| {
        leading_boundary_ok(code, start)
            && trailing_boundary_ok(code, start, "tokio::task")
            && !is_bare_task_import_at(code, &statements, start)
    })
}

fn use_statement_ranges(code: &str) -> Vec<(usize, usize)> {
    code.match_indices("use ")
        .filter(|(start, _)| use_item_prefix_allowed(code, *start))
        .filter_map(|(start, _)| use_statement_end(code, start).map(|end| (start, end)))
        .collect()
}

fn use_statement_end(code: &str, start: usize) -> Option<usize> {
    let (_, from_use) = code.split_at_checked(start)?;
    let semicolon = from_use.find(';')?;
    start.checked_add(semicolon)?.checked_add(1)
}

fn is_bare_task_import_at(code: &str, statements: &[(usize, usize)], start: usize) -> bool {
    statements
        .iter()
        .find(|(stmt_start, stmt_end)| start >= *stmt_start && start < *stmt_end)
        .is_some_and(|(stmt_start, stmt_end)| {
            statement_text(code, *stmt_start, *stmt_end).is_some_and(is_bare_task_statement)
        })
}

fn statement_text(code: &str, start: usize, end: usize) -> Option<&str> {
    let (_, from_start) = code.split_at_checked(start)?;
    let (statement, _) = from_start.split_at_checked(end.checked_sub(start)?)?;
    Some(statement)
}

fn is_bare_task_statement(statement: &str) -> bool {
    let compact: String = statement.chars().filter(|ch| !ch.is_whitespace()).collect();
    compact == "usetokio::task;"
        || compact == "usetokio::task::{self};"
        || compact == "usetokio::task::{self,};"
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
        || grouped_std_import_contains(source, &["sync::Mutex", "sync::RwLock"])
        || direct_import_contains(source, "std::cell::", &["RefCell"])
        || grouped_import_contains(source, "use std::cell::", &["RefCell"])
        || grouped_std_import_contains(source, &["cell::RefCell"])
        || grouped_import_contains(source, "use std::thread::", &["spawn"])
        || grouped_std_import_contains(source, &["thread::spawn"])
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
        || prefix.ends_with(';')
        || prefix.ends_with('{')
        || prefix.ends_with('}')
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
    let after_open = from_prefix.strip_prefix(prefix)?.trim_start().strip_prefix('{')?;
    let close = matching_close_brace(after_open)?;
    after_open.split_at_checked(close).map(|(body, _)| body)
}

fn matching_close_brace(source: &str) -> Option<usize> {
    let mut depth = 1usize;
    source.char_indices().find_map(|(index, ch)| {
        depth = brace_depth_after(depth, ch)?;
        (depth == 0).then_some(index)
    })
}

const fn brace_depth_after(depth: usize, ch: char) -> Option<usize> {
    match ch {
        '{' => depth.checked_add(1),
        '}' => depth.checked_sub(1),
        _ => Some(depth),
    }
}

fn grouped_body_contains_name(body: &str, name: &str) -> bool {
    flattened_member_paths(body).iter().any(|path| grouped_member_matches(path, name))
}

fn flattened_member_paths(body: &str) -> Vec<String> {
    top_level_members(body).iter().flat_map(|member| flatten_member(member)).collect()
}

fn top_level_members(body: &str) -> Vec<&str> {
    let (members, _, member_start) =
        body.char_indices().fold((Vec::new(), 0usize, 0usize), |state, (index, ch)| {
            advance_member_scan(body, state, index, ch)
        });
    members
        .into_iter()
        .chain(member_slice(body, member_start, body.len()))
        .map(str::trim)
        .filter(|member| !member.is_empty())
        .collect()
}

fn advance_member_scan<'a>(
    body: &'a str,
    state: (Vec<&'a str>, usize, usize),
    index: usize,
    ch: char,
) -> (Vec<&'a str>, usize, usize) {
    let (mut members, depth, member_start) = state;
    match ch {
        '{' => (members, depth.saturating_add(1), member_start),
        '}' => (members, depth.saturating_sub(1), member_start),
        ',' if depth == 0 => {
            members.extend(member_slice(body, member_start, index));
            (members, depth, index.saturating_add(1))
        }
        _ => (members, depth, member_start),
    }
}

fn member_slice(body: &str, start: usize, end: usize) -> Option<&str> {
    let (_, from_start) = body.split_at_checked(start)?;
    let (member, _) = from_start.split_at_checked(end.checked_sub(start)?)?;
    Some(member)
}

fn flatten_member(member: &str) -> Vec<String> {
    let Some(open) = member.find('{') else {
        return vec![member.trim().to_owned()];
    };
    let Some((head, inner)) = member_head_and_group(member, open) else {
        return vec![member.trim().to_owned()];
    };
    flattened_member_paths(inner).iter().map(|path| format!("{head}::{path}")).collect()
}

fn member_head_and_group(member: &str, open: usize) -> Option<(&str, &str)> {
    let (head, from_open) = member.split_at_checked(open)?;
    let inner = from_open.strip_prefix('{')?;
    let close = matching_close_brace(inner)?;
    let (body, _) = inner.split_at_checked(close)?;
    Some((head.trim().trim_end_matches("::").trim_end(), body))
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
    let definitions = alias_definitions(code);
    let resolved = (0..definitions.len())
        .fold(vec![krate.to_owned()], |current, _| extend_resolved_aliases(&definitions, &current));
    resolved.into_iter().filter(|alias| alias != krate).collect()
}

fn alias_definitions(code: &str) -> Vec<(String, String)> {
    code.match_indices("use ").filter_map(|(start, _)| alias_definition_at(code, start)).collect()
}

fn alias_definition_at(source: &str, start: usize) -> Option<(String, String)> {
    use_item_prefix_allowed(source, start).then_some(())?;
    let statement =
        source.split_at_checked(start)?.1.strip_prefix("use ")?.split_once(';')?.0.trim();
    plain_alias_definition(statement).or_else(|| grouped_alias_definition(statement))
}

fn plain_alias_definition(statement: &str) -> Option<(String, String)> {
    let (head, alias) = statement.split_once(" as ")?;
    let head = valid_alias(head.trim())?;
    valid_alias(alias.trim()).map(|alias| (head, alias))
}

fn grouped_alias_definition(statement: &str) -> Option<(String, String)> {
    let (head, grouped) = statement.split_once("::{")?;
    let head = valid_alias(head.trim())?;
    let grouped = grouped.trim().strip_suffix('}')?;
    grouped
        .split(',')
        .map(str::trim)
        .find_map(|member| member.strip_prefix("self as "))
        .and_then(|alias| valid_alias(alias.trim()))
        .map(|alias| (head, alias))
}

fn extend_resolved_aliases(definitions: &[(String, String)], resolved: &[String]) -> Vec<String> {
    definitions
        .iter()
        .filter(|(head, _)| resolved.iter().any(|name| name == head))
        .fold(resolved.to_vec(), |acc, (_, alias)| push_alias_once(acc, alias))
}

fn push_alias_once(mut acc: Vec<String>, alias: &String) -> Vec<String> {
    if !acc.contains(alias) {
        acc.push(alias.clone());
    }
    acc
}

fn valid_alias(alias: &str) -> Option<String> {
    (!alias.is_empty() && alias.chars().all(is_ident_char)).then(|| alias.to_owned())
}
