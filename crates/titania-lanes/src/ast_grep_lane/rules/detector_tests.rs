use super::detectors::{
    detect_core_async_effect, detect_core_fs_import, detect_core_infra_import,
    detect_core_process_import, detect_core_random_import, detect_core_state_effect,
    detect_core_time_import,
};

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

#[test]
fn tn_bvv7_fq_infra_without_use_is_violation() {
    assert!(detect_core_infra_import("pub fn fetch() { tokio::spawn(async { 1 }); }"));
    assert!(!detect_core_infra_import("// tokio::spawn mentioned in a comment"));
    assert!(!detect_core_infra_import(r#"let label = "tokio::spawn";"#));
}

#[test]
fn tn_bvv7_fq_fs_without_use_is_violation() {
    assert!(detect_core_fs_import(
        "pub fn load() -> String { std::fs::read_to_string(\"c\").unwrap_or_default() }"
    ));
    assert!(detect_core_fs_import(
        "pub fn host() -> String { std::env::var(\"H\").unwrap_or_default() }"
    ));
    assert!(detect_core_fs_import(
        "macro_rules! load { () => { std::fs::read_to_string(\"c\") } }"
    ));
    assert!(!detect_core_fs_import("// std::fs::read in a comment"));
}

#[test]
fn tn_bvv7_fq_time_without_use_is_violation() {
    assert!(detect_core_time_import(
        "pub fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_secs()) }"
    ));
    assert!(detect_core_time_import(
        "pub fn start() -> std::time::Instant { std::time::Instant::now() }"
    ));
    assert!(!detect_core_time_import("// std::time::Instant in a comment"));
}

#[test]
fn tn_bvv7_fq_random_without_use_is_violation() {
    assert!(detect_core_random_import("pub fn pick() -> u8 { rand::thread_rng().gen() }"));
    assert!(!detect_core_random_import("// rand::thread_rng in a comment"));
}

#[test]
fn tn_bvv7_aliased_fs_import_stays_violation() {
    assert!(detect_core_fs_import("use std::fs as filesystem;"));
    assert!(detect_core_fs_import("use std::{env as environment, fs};"));
}

#[test]
fn tn_bvv7_boundary_names_do_not_fire() {
    assert!(!detect_core_fs_import("use fs_extra::dir;"));
    assert!(!detect_core_fs_import("pub fn copy() { fs_extra::dir::copy(\"a\", \"b\"); }"));
    assert!(!detect_core_random_import("pub fn x() -> u8 { brand::thread_rng() }"));
    assert!(!detect_core_random_import("use rand_core::RngCore;"));
}

#[test]
fn tn_bvv7_process_import_is_violation() {
    assert!(detect_core_process_import("use std::process::Command;"));
    assert!(detect_core_process_import("use std::process as proc;"));
    assert!(detect_core_process_import("use std::{process, collections};"));
    assert!(detect_core_process_import(
        "pub fn sh() { std::process::Command::new(\"ls\").spawn(); }"
    ));
    assert!(detect_core_process_import(
        "pub fn sh() { tokio::process::Command::new(\"ls\").spawn(); }"
    ));
    assert!(!detect_core_process_import("// use std::process::Command"));
    assert!(!detect_core_process_import(r#"let s = "std::process::Command";"#));
    assert!(!detect_core_process_import(
        "use std::collections::HashMap;\npub fn compute(x: i32) -> i32 { x + 1 }"
    ));
}

#[test]
fn tn_bvv7_async_effect_is_violation() {
    assert!(detect_core_async_effect("pub async fn fetch() -> i32 { 1 }"));
    assert!(detect_core_async_effect(
        "pub fn run(fut: impl Future<Output = i32>) -> i32 { fut.await }"
    ));
    assert!(detect_core_async_effect("pub fn go() { tokio::spawn(async { 1 }); }"));
    assert!(detect_core_async_effect("use tokio::task::JoinSet;"));
    assert!(!detect_core_async_effect("// async fn fetch() in a comment"));
    assert!(!detect_core_async_effect(r#"let s = "async fn";"#));
    assert!(!detect_core_async_effect("pub fn compute(x: i32) -> i32 { x + 1 }"));
}

#[test]
fn tn_bvv7_state_effect_is_violation() {
    assert!(detect_core_state_effect("static mut COUNTER: i32 = 0;"));
    assert!(detect_core_state_effect(
        "use std::sync::Mutex;\nstatic HITS: Mutex<i32> = Mutex::new(0);"
    ));
    assert!(detect_core_state_effect("pub fn f(m: &std::sync::RwLock<i32>) -> i32 { 1 }"));
    assert!(detect_core_state_effect(
        "use std::cell::RefCell;\nthread_local! { static C: RefCell<i32> = const { RefCell::new(0) }; }"
    ));
    assert!(detect_core_state_effect("pub fn bg() { std::thread::spawn(|| 1); }"));
}

#[test]
fn tn_bvv7_state_effect_allows_frozen_catalog_pattern() {
    assert!(!detect_core_state_effect("use std::sync::LazyLock;"));
    assert!(!detect_core_state_effect(
        "static ROWS: LazyLock<Vec<i32>> = LazyLock::new(|| Vec::new());"
    ));
    assert!(!detect_core_state_effect("const TSV: &str = include_str!(\"repair_catalog.tsv\");"));
    assert!(!detect_core_state_effect(
        "use std::sync::OnceLock;\nstatic CELL: OnceLock<i32> = OnceLock::new();"
    ));
    assert!(!detect_core_state_effect("// static mut COUNTER in a comment"));
}

use proptest::prelude::*;

fn padding() -> impl Strategy<Value = String> {
    prop::collection::vec("[ab \n;]{0,24}", 0..3).prop_map(|chunks| chunks.join("\n"))
}

fn embed(pad: &str, line: &str) -> String {
    format!("{pad}\n{line}\n{pad}")
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    #[test]
    fn tn_bvv7_fq_bypass_property(pad in padding(), pick in 0..8usize) {
        type Detector = fn(&str) -> bool;
        let pool: [(Detector, &str); 8] = [
            (detect_core_fs_import, "std::fs::read_to_string(\"c\")"),
            (detect_core_fs_import, "use std::fs as filesystem;"),
            (
                detect_core_fs_import,
                "macro_rules! m { () => { std::env::var(\"H\") } }",
            ),
            (detect_core_time_import, "std::time::SystemTime::now()"),
            (detect_core_random_import, "rand::thread_rng()"),
            (detect_core_infra_import, "tokio::spawn(async { 1 })"),
            (detect_core_process_import, "std::process::Command::new(\"ls\")"),
            (detect_core_async_effect, "pending.await"),
        ];
        let (detect, line) = pool[pick];
        prop_assert!(!detect(&pad));
        prop_assert!(detect(&embed(&pad, line)));
    }

    #[test]
    fn tn_bvv7_effect_boundary_property(pad in padding(), pick in 0..9usize) {
        type Detector = fn(&str) -> bool;
        let pool: [(Detector, &str, bool); 9] = [
            (detect_core_process_import, "use std::process::Command;", true),
            (detect_core_async_effect, "pub async fn f() -> i32 { 1 }", true),
            (detect_core_async_effect, "tokio::task::JoinSet::new()", true),
            (detect_core_state_effect, "static mut C: i32 = 0;", true),
            (
                detect_core_state_effect,
                "use std::sync::Mutex;",
                true,
            ),
            (detect_core_state_effect, "std::thread::spawn(|| 1)", true),
            (
                detect_core_state_effect,
                "use std::sync::LazyLock;",
                false,
            ),
            (
                detect_core_state_effect,
                "static R: LazyLock<i32> = LazyLock::new(|| 1);",
                false,
            ),
            (
                detect_core_state_effect,
                "const T: &str = include_str!(\"f.tsv\");",
                false,
            ),
        ];
        let (detect, line, expected) = pool[pick];
        prop_assert!(!detect(&pad));
        prop_assert_eq!(detect(&embed(&pad, line)), expected);
    }
}
