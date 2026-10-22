use super::detectors::{
    detect_core_async_effect, detect_core_fs_import, detect_core_infra_import,
    detect_core_process_import, detect_core_random_import, detect_core_state_effect,
    detect_core_time_import,
};
use super::super::span::MatchSite;

#[test]
fn detect_core_infra_import_catches_real_use() {
    assert!(!detect_core_infra_import("use tokio::task;").is_empty());
    assert!(!detect_core_infra_import("use axum::Router;").is_empty());
    assert!(!detect_core_infra_import("use sqlx::Migrator;").is_empty());
    assert!(!detect_core_infra_import("use reqwest::Client;").is_empty());
}

#[test]
fn detect_core_infra_import_skips_comment_mentions() {
    assert!(detect_core_infra_import("// use tokio::task").is_empty());
    assert!(detect_core_infra_import("/* use axum::Router */").is_empty());
    assert!(detect_core_infra_import("#[doc = \"use sqlx::Migrator\"]").is_empty());
    assert!(detect_core_infra_import("/// Requires `use reqwest::Client;`").is_empty());
}

#[test]
fn detect_core_infra_import_skips_string_literal_mentions() {
    assert!(detect_core_infra_import(r#""use tokio::task;""#).is_empty());
    assert!(detect_core_infra_import(r#""use axum::Router""#).is_empty());
}

#[test]
fn detect_core_infra_import_allows_clean_code() {
    let clean = r#"
use std::collections::HashMap;

pub fn compute(x: i32) -> i32 {
x + 1
}
"#;
    assert!(detect_core_infra_import(clean).is_empty());
}

#[test]
fn tn_bvv7_fq_infra_without_use_is_violation() {
    assert!(!detect_core_infra_import("pub fn fetch() { tokio::spawn(async { 1 }); }").is_empty());
    assert!(detect_core_infra_import("// tokio::spawn mentioned in a comment").is_empty());
    assert!(detect_core_infra_import(r#"let label = "tokio::spawn";"#).is_empty());
}

#[test]
fn tn_bvv7_fq_fs_without_use_is_violation() {
    assert!(!detect_core_fs_import(
        "pub fn load() -> String { std::fs::read_to_string(\"c\").unwrap_or_default() }"
    ).is_empty());
    assert!(!detect_core_fs_import(
        "pub fn host() -> String { std::env::var(\"H\").unwrap_or_default() }"
    ).is_empty());
    assert!(!detect_core_fs_import(
        "macro_rules! load { () => { std::fs::read_to_string(\"c\") } }"
    ).is_empty());
    assert!(detect_core_fs_import("// std::fs::read in a comment").is_empty());
}

#[test]
fn tn_bvv7_fq_time_without_use_is_violation() {
    assert!(!detect_core_time_import(
        "pub fn now() -> u64 { std::time::SystemTime::now().duration_since(std::time::SystemTime::UNIX_EPOCH).map_or(0, |d| d.as_secs()) }"
    ).is_empty());
    assert!(!detect_core_time_import(
        "pub fn start() -> std::time::Instant { std::time::Instant::now() }"
    ).is_empty());
    assert!(detect_core_time_import("// std::time::Instant in a comment").is_empty());
}

#[test]
fn tn_bvv7_fq_random_without_use_is_violation() {
    assert!(!detect_core_random_import("pub fn pick() -> u8 { rand::thread_rng().gen() }").is_empty());
    assert!(detect_core_random_import("// rand::thread_rng in a comment").is_empty());
}

#[test]
fn tn_bvv7_aliased_fs_import_stays_violation() {
    assert!(!detect_core_fs_import("use std::fs as filesystem;").is_empty());
    assert!(!detect_core_fs_import("use std::{env as environment, fs};").is_empty());
}

#[test]
fn tn_bvv7_boundary_names_do_not_fire() {
    assert!(detect_core_fs_import("use fs_extra::dir;").is_empty());
    assert!(detect_core_fs_import("pub fn copy() { fs_extra::dir::copy(\"a\", \"b\"); }").is_empty());
    assert!(detect_core_random_import("pub fn x() -> u8 { brand::thread_rng() }").is_empty());
    assert!(detect_core_random_import("use rand_core::RngCore;").is_empty());
}

#[test]
fn tn_bvv7_process_import_is_violation() {
    assert!(!detect_core_process_import("use std::process::Command;").is_empty());
    assert!(!detect_core_process_import("use std::process as proc;").is_empty());
    assert!(!detect_core_process_import("use std::{process, collections};").is_empty());
    assert!(!detect_core_process_import(
        "pub fn sh() { std::process::Command::new(\"ls\").spawn(); }"
    ).is_empty());
    assert!(!detect_core_process_import(
        "pub fn sh() { tokio::process::Command::new(\"ls\").spawn(); }"
    ).is_empty());
    assert!(detect_core_process_import("// use std::process::Command").is_empty());
    assert!(detect_core_process_import(r#"let s = "std::process::Command";"#).is_empty());
    assert!(detect_core_process_import(
        "use std::collections::HashMap;\npub fn compute(x: i32) -> i32 { x + 1 }"
    ).is_empty());
}

#[test]
fn tn_bvv7_async_effect_is_violation() {
    assert!(!detect_core_async_effect("pub async fn fetch() -> i32 { 1 }").is_empty());
    assert!(!detect_core_async_effect(
        "pub fn run(fut: impl Future<Output = i32>) -> i32 { fut.await }"
    ).is_empty());
    assert!(!detect_core_async_effect("pub fn go() { tokio::spawn(async { 1 }); }").is_empty());
    assert!(!detect_core_async_effect("use tokio::task::JoinSet;").is_empty());
    assert!(detect_core_async_effect("// async fn fetch() in a comment").is_empty());
    assert!(detect_core_async_effect(r#"let s = "async fn";"#).is_empty());
    assert!(detect_core_async_effect("pub fn compute(x: i32) -> i32 { x + 1 }").is_empty());
}

#[test]
fn tn_bvv7_state_effect_is_violation() {
    assert!(!detect_core_state_effect("static mut COUNTER: i32 = 0;").is_empty());
    assert!(!detect_core_state_effect(
        "use std::sync::Mutex;\nstatic HITS: Mutex<i32> = Mutex::new(0);"
    ).is_empty());
    assert!(!detect_core_state_effect("pub fn f(m: &std::sync::RwLock<i32>) -> i32 { 1 }").is_empty());
    assert!(!detect_core_state_effect(
        "use std::cell::RefCell;\nthread_local! { static C: RefCell<i32> = const { RefCell::new(0) }; }"
    ).is_empty());
    assert!(!detect_core_state_effect("pub fn bg() { std::thread::spawn(|| 1); }").is_empty());
}

#[test]
fn tn_bvv7_state_effect_allows_frozen_catalog_pattern() {
    assert!(detect_core_state_effect("use std::sync::LazyLock;").is_empty());
    assert!(detect_core_state_effect(
        "static ROWS: LazyLock<Vec<i32>> = LazyLock::new(|| Vec::new());"
    ).is_empty());
    assert!(detect_core_state_effect("const TSV: &str = include_str!(\"repair_catalog.tsv\");").is_empty());
    assert!(detect_core_state_effect(
        "use std::sync::OnceLock;\nstatic CELL: OnceLock<i32> = OnceLock::new();"
    ).is_empty());
    assert!(detect_core_state_effect("// static mut COUNTER in a comment").is_empty());
}

#[test]
fn audit6_random_injected_rng_trait_is_allowed() {
    assert!(detect_core_random_import("use rand::Rng;").is_empty());
    assert!(detect_core_random_import(
        "pub fn choose<R: Rng>(rng: &mut R, len: usize) -> usize { rng.gen_range(0..len) }"
    ).is_empty());
    assert!(!detect_core_random_import("use rand::thread_rng;").is_empty());
    assert!(!detect_core_random_import("use rand::{Rng, thread_rng};").is_empty());
}

#[test]
fn audit6_state_grouped_imports_are_violations() {
    assert!(!detect_core_state_effect(
        "use std::sync::{Mutex, RwLock};\nstatic HITS: Mutex<i32> = Mutex::new(0);"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "use std::cell::{RefCell};\nthread_local! { static C: RefCell<i32> = const { RefCell::new(0) }; }"
    ).is_empty());
    assert!(!detect_core_state_effect("use std::thread::{spawn};\npub fn bg() { spawn(|| 1); }").is_empty());
}

#[test]
fn audit6_state_beyond_spec_constructs_are_allowed() {
    assert!(detect_core_state_effect("pub fn f(cell: &std::cell::Cell<i32>) {}").is_empty());
    assert!(detect_core_state_effect("pub fn f(cell: &std::cell::UnsafeCell<i32>) {}").is_empty());
    assert!(detect_core_state_effect("use std::cell::Cell;\npub fn f(cell: &Cell<i32>) {}").is_empty());
    assert!(detect_core_state_effect(
        "pub fn nap() { std::thread::sleep(std::time::Duration::from_secs(1)); }"
    ).is_empty());
}

#[test]
fn audit6_async_field_named_await_does_not_fire() {
    assert!(detect_core_async_effect("pub fn count(stats: &Stats) -> u32 { stats.await_count }").is_empty());
    assert!(detect_core_async_effect("pub fn count(stats: &Stats) -> u32 { stats.awaiting }").is_empty());
    assert!(!detect_core_async_effect(
        "pub fn run(fut: impl Future<Output = i32>) -> i32 { fut.await }"
    ).is_empty());
}

#[test]
fn audit6_async_unsafe_fn_is_violation() {
    assert!(!detect_core_async_effect("pub async unsafe fn fetch() -> i32 { 1 }").is_empty());
}

#[test]
fn audit6_bare_tokio_task_import_alone_is_not_async_effect() {
    assert!(detect_core_async_effect("use tokio::task;").is_empty());
    assert!(detect_core_async_effect("pub use tokio::task;").is_empty());
    assert!(!detect_core_async_effect("use tokio::task::JoinSet;").is_empty());
    assert!(!detect_core_async_effect("pub fn go() { tokio::task::spawn_local(async { 1 }); }").is_empty());
}

#[test]
fn audit6_crate_root_alias_infra_is_violation() {
    assert!(!detect_core_infra_import("use tokio as tk;\npub fn go() { tk::spawn(async { 1 }); }").is_empty());
    assert!(!detect_core_infra_import(
        "use tokio::{self as tk};\npub fn go() { tk::spawn(async { 1 }); }"
    ).is_empty());
    assert!(!detect_core_async_effect("use tokio as tk;\npub fn go() { tk::spawn(async { 1 }); }").is_empty());
    assert!(detect_core_infra_import("use tokio as tk;\npub fn go() -> i32 { 1 }").is_empty());
    assert!(detect_core_infra_import(
        "use tokio_util as tk;\npub fn go() { tk::spawn(async { 1 }); }"
    ).is_empty());
}

#[test]
fn bh2_state_nested_group_imports_are_violations() {
    assert!(!detect_core_state_effect(
        "use std::{sync::Mutex};\nstatic HITS: Mutex<i32> = Mutex::new(0);"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "use std::{sync::{Mutex, RwLock}};\nstatic HITS: Mutex<i32> = Mutex::new(0);"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "use std::{cell::RefCell};\nthread_local! { static C: RefCell<i32> = const { RefCell::new(0) }; }"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "use std::{cell::{RefCell}};\nthread_local! { static C: RefCell<i32> = const { RefCell::new(0) }; }"
    ).is_empty());
    assert!(!detect_core_state_effect("use std::{thread::spawn};\npub fn bg() { spawn(|| 1); }").is_empty());
    assert!(!detect_core_state_effect("use std::{thread::{spawn}};\npub fn bg() { spawn(|| 1); }").is_empty());
    assert!(!detect_core_state_effect(
        "use std::{sync::Mutex, time::Instant};\npub fn f(m: Mutex<i32>) -> Mutex<i32> { m }"
    ).is_empty());
}

#[test]
fn bh2_time_nested_group_imports_are_violations() {
    assert!(!detect_core_time_import(
        "use std::{time::Instant};\npub fn start() -> Instant { Instant::now() }"
    ).is_empty());
    assert!(!detect_core_time_import(
        "use std::{time::{SystemTime, Instant}};\npub fn start() -> Instant { Instant::now() }"
    ).is_empty());
}

#[test]
fn bh2_nested_group_neighbor_imports_stay_allowed() {
    assert!(detect_core_state_effect("use std::{sync::LazyLock};").is_empty());
    assert!(detect_core_state_effect("use std::{sync::MutexGuard};").is_empty());
    assert!(detect_core_state_effect("use std::{cell::Cell};").is_empty());
    assert!(detect_core_state_effect("use std::{collections::HashMap};").is_empty());
    assert!(detect_core_time_import("use std::{time::Duration};").is_empty());
    assert!(detect_core_time_import("use std::{time::Instantaneous};").is_empty());
}

#[test]
fn bh2_alias_chain_is_violation() {
    assert!(!detect_core_infra_import(
        "use tokio as tk;\nuse tk as t2;\npub fn go() { t2::spawn(async { 1 }); }"
    ).is_empty());
    assert!(!detect_core_async_effect(
        "use tokio as tk;\nuse tk as t2;\npub fn go() { t2::spawn(async { 1 }); }"
    ).is_empty());
    assert!(!detect_core_infra_import(
        "use tokio as tk; use tk as t2; pub fn go() { t2::spawn(async { 1 }); }"
    ).is_empty());
    assert!(!detect_core_infra_import(
        "use tokio as a;\nuse a as b;\nuse b as c;\npub fn go() { c::task::spawn(async { 1 }); }"
    ).is_empty());
    assert!(!detect_core_infra_import(
        "use tokio::{self as tk};\nuse tk as t2;\npub fn go() { t2::spawn(async { 1 }); }"
    ).is_empty());
    assert!(!detect_core_infra_import(
        "use tokio as tk;\nuse tk::{self as t2};\npub fn go() { t2::spawn(async { 1 }); }"
    ).is_empty());
    assert!(!detect_core_infra_import(
        "use tokio as tk;\nuse tk as t2;\nuse t2 as tk;\npub fn go() { t2::spawn(async { 1 }); }"
    ).is_empty());
}

#[test]
fn bh2_alias_chain_requires_tokio_root() {
    assert!(detect_core_infra_import(
        "use tokio_util as tk;\nuse tk as t2;\npub fn go() { t2::spawn(async { 1 }); }"
    ).is_empty());
    assert!(detect_core_infra_import("use tk as t2;\npub fn go() { t2::spawn(async { 1 }); }").is_empty());
    assert!(detect_core_async_effect("use tokio as tk;\nuse tk as t2;\npub fn go() -> i32 { 1 }").is_empty());
}

#[test]
fn bh3_module_self_import_state_is_violation() {
    assert!(!detect_core_state_effect(
        "use std::sync::{self};\npub fn f() { let _m = sync::Mutex::new(0); }"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "use std::{sync::{self}};\npub fn f() { let _m = sync::Mutex::new(0); }"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "pub use std::sync::{self};\npub fn f() { let _m = sync::Mutex::new(0); }"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "use std::sync;\npub fn f() { let _m = sync::Mutex::new(0); }"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "use std::sync::{self};\npub fn f(m: &sync::RwLock<i32>) -> i32 { 1 }"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "use std::cell::{self};\npub fn f() { let _c = cell::RefCell::new(0); }"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "use std::thread::{self};\npub fn f() { let _h = thread::spawn(|| 1); }"
    ).is_empty());
}

#[test]
fn bh3_module_self_import_time_is_violation() {
    assert!(!detect_core_time_import(
        "use std::time::{self};\npub fn f() { let _t = time::Instant::now(); }"
    ).is_empty());
    assert!(!detect_core_time_import(
        "use std::{time::{self}};\npub fn f() { let _t = time::SystemTime::now(); }"
    ).is_empty());
    assert!(!detect_core_time_import(
        "use std::time;\npub fn f() { let _t = time::Instant::now(); }"
    ).is_empty());
}

#[test]
fn bh3_module_renamed_binding_is_violation() {
    assert!(!detect_core_state_effect(
        "use std::sync as s;\npub fn f() { let _m = s::Mutex::new(0); }"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "use std::sync::{self as s};\npub fn f() { let _m = s::RwLock::new(0); }"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "use std::{sync as s};\npub fn f() { let _m = s::Mutex::new(0); }"
    ).is_empty());
    assert!(!detect_core_state_effect(
        "use std::{sync::{self as s}};\npub fn f() { let _m = s::Mutex::new(0); }"
    ).is_empty());
}

#[test]
fn bh3_module_binding_requires_member_use() {
    assert!(detect_core_state_effect("use std::sync::{self};\npub fn f() -> i32 { 1 }").is_empty());
    assert!(detect_core_time_import("use std::time::{self};\npub fn f() -> i32 { 1 }").is_empty());
    assert!(detect_core_state_effect(
        "use std::sync::{self};\npub fn f() { let _l = sync::LazyLock::new(|| 1); }"
    ).is_empty());
    assert!(detect_core_state_effect(
        "use std::sync::{self};\npub fn f(g: sync::MutexGuard<'_, i32>) {}"
    ).is_empty());
    assert!(detect_core_time_import(
        "use std::time::{self};\npub fn f() { let _d = time::Duration::from_secs(1); }"
    ).is_empty());
}

#[test]
fn bh3_module_shadow_does_not_fire() {
    assert!(detect_core_state_effect(
        "mod sync { pub struct Mutex; }\npub fn f() { let _m = sync::Mutex::new(0); }"
    ).is_empty());
    assert!(detect_core_state_effect(
        "use mycrate::sync;\npub fn f() { let _m = sync::Mutex::new(0); }"
    ).is_empty());
    assert!(detect_core_state_effect(
        "use crate::sync::{self};\npub fn f() { let _m = sync::Mutex::new(0); }"
    ).is_empty());
    assert!(detect_core_state_effect(
        "use std::sync::{self};\npub fn f() { let _m = other::sync::Mutex::new(0); }"
    ).is_empty());
}

#[test]
fn bh3_module_self_import_fs_process_stay_violations() {
    assert!(!detect_core_fs_import(
        "use std::fs::{self};\npub fn load() { let _c = fs::read_to_string(\"c\"); }"
    ).is_empty());
    assert!(!detect_core_fs_import(
        "use std::fs;\npub fn load() { let _c = fs::read_to_string(\"c\"); }"
    ).is_empty());
    assert!(!detect_core_process_import(
        "use std::process::{self};\npub fn sh() { let _c = process::Command::new(\"ls\"); }"
    ).is_empty());
}

#[test]
fn bh3_bare_task_redundant_alias_is_not_async_effect() {
    assert!(detect_core_async_effect("use tokio::task::{self as task};").is_empty());
    assert!(detect_core_async_effect("use tokio::task as task;").is_empty());
    assert!(!detect_core_infra_import("use tokio::task::{self as task};").is_empty());
    assert!(!detect_core_infra_import("use tokio::task as task;").is_empty());
    assert!(!detect_core_async_effect("use tokio::task::{self as task, spawn};").is_empty());
    assert!(!detect_core_async_effect("use tokio::task as other;").is_empty());
}

#[test]
fn bh3_grouped_tokio_task_spawn_is_async_effect() {
    assert!(!detect_core_async_effect(
        "use tokio::{task::spawn};\npub fn go() { spawn(async { 1 }); }"
    ).is_empty());
    assert!(!detect_core_async_effect(
        "use tokio::{task::{spawn}};\npub fn go() { spawn(async { 1 }); }"
    ).is_empty());
}

#[test]
fn bh2_bare_task_import_spellings_are_not_async_effect() {
    assert!(detect_core_async_effect("use tokio::task::{\n    self,\n};\npub fn f() {}").is_empty());
    assert!(detect_core_async_effect("use tokio::task::{\n    self\n};\npub fn f() {}").is_empty());
    assert!(detect_core_async_effect("pub use tokio::task::{\n    self,\n};\npub fn f() {}").is_empty());
    assert!(detect_core_async_effect("use tokio::task; pub fn f() {}").is_empty());
    assert!(!detect_core_async_effect("use tokio::task::{\n    self,\n    spawn,\n};").is_empty());
    assert!(!detect_core_async_effect("use tokio::task::{\n    JoinSet,\n};").is_empty());
}

#[test]
fn audit6_char_literal_quote_does_not_hide_following_code() {
    assert!(!detect_core_fs_import(
        "pub const QUOTE: char = '\"';\npub fn load() -> String { std::fs::read_to_string(\"c\").unwrap_or_default() }"
    ).is_empty());
    assert!(!detect_core_fs_import(
        "pub const BACKSLASH: char = '\\\\';\npub fn load() -> String { std::fs::read_to_string(\"c\").unwrap_or_default() }"
    ).is_empty());
    assert!(detect_core_fs_import("pub fn id<'a>(value: &'a str) -> &'a str { value }").is_empty());
    assert!(detect_core_fs_import(
        "pub const QUOTE: char = '\"';\npub fn pure(x: i32) -> i32 { x + 1 }"
    ).is_empty());
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
        type Detector = fn(&str) -> Vec<MatchSite>;
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
        prop_assert!(detect(&pad).is_empty());
        prop_assert!(!detect(&embed(&pad, line)).is_empty());
    }

    #[test]
    fn tn_bvv7_effect_boundary_property(pad in padding(), pick in 0..9usize) {
        type Detector = fn(&str) -> Vec<MatchSite>;
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
        prop_assert!(detect(&pad).is_empty());
        prop_assert_eq!(!detect(&embed(&pad, line)).is_empty(), expected);
    }
}

#[test]
fn architecture_sites_cover_every_occurrence() {
    let source = "pub fn a() { std::fs::read(\"x\"); }\npub fn b() { std::fs::write(\"y\", \"z\"); }";
    let sites = detect_core_fs_import(source);
    assert_eq!(sites.len(), 2);
    for site in &sites {
        assert_eq!(&source[site.start_byte..site.end_byte], "std::fs::");
    }
}

#[test]
fn architecture_sites_dedup_overlapping_channels() {
    let source = "use std::fs::File;";
    let sites = detect_core_fs_import(source);
    assert_eq!(sites.len(), 1);
    assert_eq!(&source[sites[0].start_byte..sites[0].end_byte], "std::fs::");
}

#[test]
fn architecture_sites_split_grouped_import_members() {
    let source = "use std::{env, fs};";
    let sites = detect_core_fs_import(source);
    let texts: Vec<&str> =
        sites.iter().map(|site| &source[site.start_byte..site.end_byte]).collect();
    assert_eq!(texts, ["env", "fs"]);
}

#[test]
fn architecture_sites_keep_positions_after_strings() {
    let source = "let label = \"tokio::spawn\";\npub fn go() { tokio::spawn(async { 1 }); }";
    let sites = detect_core_infra_import(source);
    assert_eq!(sites.len(), 1);
    assert_eq!(&source[sites[0].start_byte..sites[0].end_byte], "tokio::");
    assert_eq!(source.rfind("tokio::"), Some(sites[0].start_byte));
}

#[test]
fn architecture_infra_sites_cover_distinct_lines() {
    let sites = detect_core_infra_import("use tokio::task;\nuse axum::Router;");
    assert_eq!(sites.len(), 2);
}

#[test]
fn architecture_async_sites_cover_each_construct() {
    let source = "pub async fn a() {}\npub fn b() { tokio::spawn(async { 1 }); }\npub fn c(f: impl Future<Output = i32>) -> i32 { f.await }";
    let sites = detect_core_async_effect(source);
    assert_eq!(sites.len(), 3);
}

#[test]
fn architecture_state_import_and_use_site_dedup() {
    let source = "use std::sync::Mutex;\nstatic HITS: Mutex<i32> = Mutex::new(0);";
    let sites = detect_core_state_effect(source);
    assert_eq!(sites.len(), 1);
    assert_eq!(&source[sites[0].start_byte..sites[0].end_byte], "std::sync::Mutex");
}

#[test]
fn architecture_sites_stay_aligned_with_crlf_line_endings() {
    let source = "pub fn a() {}\r\npub fn b() { std::fs::read(\"x\"); }";
    let sites = detect_core_fs_import(source);
    assert_eq!(sites.len(), 1);
    assert_eq!(&source[sites[0].start_byte..sites[0].end_byte], "std::fs::");
}
