use super::*;
use std::ffi::OsString;
#[cfg(unix)]
use std::os::unix::ffi::OsStringExt;

fn pairs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
    pairs.iter().map(|(k, v)| ((*k).to_owned(), (*v).to_owned())).collect()
}

#[test]
fn default_scrub_keeps_only_allowlisted_keys() {
    let parent = pairs(&[
        ("PATH", "/usr/bin"),
        ("TERM", "xterm-256color"),
        ("RUSTFLAGS", "--allow-unsafe"),
        ("LD_PRELOAD", "/tmp/x.so"),
        ("CARGO_HOME", "/home/me/.cargo"),
        ("CARGO_TARGET_DIR", "/tmp/poisoned"),
        ("FOO", "bar"),
        ("HTTP_PROXY", "http://proxy:8080"),
    ]);
    let scrubbed = ScrubbedEnv::from_iter(parent);
    let kept: Vec<&str> = scrubbed.pairs().iter().map(|(k, _)| k.as_str()).collect();
    assert!(kept.contains(&"PATH"), "PATH must survive: {kept:?}");
    assert!(kept.contains(&"TERM"), "TERM must survive: {kept:?}");
    assert!(kept.contains(&"HTTP_PROXY"), "HTTP_PROXY must survive: {kept:?}");
    assert!(!kept.contains(&"RUSTFLAGS"), "RUSTFLAGS must be dropped: {kept:?}");
    assert!(!kept.contains(&"LD_PRELOAD"), "LD_PRELOAD must be dropped: {kept:?}");
    assert!(!kept.contains(&"CARGO_HOME"), "CARGO_HOME must be dropped: {kept:?}");
    assert!(!kept.contains(&"CARGO_TARGET_DIR"), "CARGO_* prefix must be dropped: {kept:?}");
    assert!(!kept.contains(&"FOO"), "non-allowlisted keys must be dropped: {kept:?}");
}

#[test]
fn default_scrub_preserves_first_occurrence_wins() {
    let parent = pairs(&[("PATH", "/first"), ("PATH", "/second")]);
    let scrubbed = ScrubbedEnv::from_iter(parent);
    let path_value = scrubbed
        .pairs()
        .iter()
        .find(|(k, _)| k == "PATH")
        .map(|(_, v)| v.as_str())
        .expect("PATH present");
    assert_eq!(path_value, "/first");
    assert_eq!(scrubbed.pairs().iter().filter(|(k, _)| k == "PATH").count(), 1);
}

#[test]
fn nul_keys_are_dropped() {
    let parent = pairs(&[("BAD\0KEY", "x"), ("PATH", "/usr/bin")]);
    let scrubbed = ScrubbedEnv::from_iter(parent);
    assert_eq!(scrubbed.len(), 1);
    assert_eq!(scrubbed.pairs()[0].0, "PATH");
}

#[test]
fn from_iter_with_policy_is_explicit() {
    let parent = pairs(&[("PATH", "/usr/bin"), ("CUSTOM", "yes"), ("RUSTFLAGS", "--drop-me")]);
    let scrubbed =
        ScrubbedEnv::from_iter_with_policy(parent, &["PATH", "CUSTOM"], &["RUSTFLAGS"], &[]);
    let kept: Vec<&str> = scrubbed.pairs().iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(kept, vec!["PATH", "CUSTOM"]);
}

#[test]
fn from_iter_accepts_string_pairs() {
    let parent: Vec<(String, String)> = vec![
        (String::from("PATH"), String::from("/usr/bin")),
        (String::from("RUSTFLAGS"), String::from("--danger")),
    ];
    let scrubbed = ScrubbedEnv::from_iter(parent);
    assert_eq!(scrubbed.len(), 1);
    assert_eq!(scrubbed.pairs()[0].0, "PATH");
}

#[test]
fn from_parent_pure_fixture_filters_known_hazards() {
    let snapshot: Vec<(OsString, OsString)> = vec![
        (OsString::from("PATH"), OsString::from("/usr/bin")),
        (OsString::from("TERM"), OsString::from("xterm-256color")),
        (OsString::from("RUSTFLAGS"), OsString::from("--danger-flag-from-parent")),
        (OsString::from("CARGO_ENCODED_RUSTFLAGS"), OsString::from("-D warnings")),
        (OsString::from("RUSTC_BOOTSTRAP"), OsString::from("1")),
        (OsString::from("RUSTC_WRAPPER"), OsString::from("/opt/sccache")),
        (OsString::from("CARGO_HOME"), OsString::from("/opt/cargo")),
        (OsString::from("CARGO_TARGET_DIR"), OsString::from("/tmp/target")),
        (OsString::from("LD_PRELOAD"), OsString::from("/tmp/x.so")),
        (OsString::from("DYLD_INSERT_LIBRARIES"), OsString::from("/tmp/y.dylib")),
        (OsString::from("HTTP_PROXY"), OsString::from("http://proxy:8080")),
        (OsString::from("FOO"), OsString::from("bar")),
    ];
    let scrubbed = ScrubbedEnv::from_parent(snapshot);
    let kept: Vec<&str> = scrubbed.pairs().iter().map(|(k, _)| k.as_str()).collect();
    assert!(kept.contains(&"PATH"), "PATH must survive; kept={kept:?}");
    assert!(kept.contains(&"TERM"), "TERM must survive; kept={kept:?}");
    assert!(kept.contains(&"HTTP_PROXY"), "HTTP_PROXY must survive; kept={kept:?}");
    for hazard in [
        "RUSTFLAGS",
        "CARGO_ENCODED_RUSTFLAGS",
        "RUSTC_BOOTSTRAP",
        "RUSTC_WRAPPER",
        "CARGO_HOME",
        "CARGO_TARGET_DIR",
        "LD_PRELOAD",
        "DYLD_INSERT_LIBRARIES",
    ] {
        assert!(!kept.contains(&hazard), "hazard {hazard} must be scrubbed; kept={kept:?}");
    }
    assert!(!kept.contains(&"FOO"), "non-allowlisted keys must be scrubbed; kept={kept:?}");
}

#[test]
fn from_parent_for_target_preserves_only_validated_lane_paths() {
    let root = std::path::Path::new("/workspace/project");
    let snapshot = vec![
        (OsString::from("CARGO_TARGET_DIR"), OsString::from(".titania/cache/clippy")),
        (
            OsString::from("CARGO_HOME"),
            OsString::from("/workspace/project/.titania/hermetic/cargo-home"),
        ),
        (
            OsString::from("RUSTUP_HOME"),
            OsString::from("/workspace/project/.titania/hermetic/rustup-home"),
        ),
        (OsString::from("CARGO_HOME"), OsString::from("/tmp/untrusted-cargo-home")),
        (OsString::from("RUSTUP_HOME"), OsString::from("/tmp/untrusted-rustup-home")),
        (OsString::from("CARGO_TARGET_DIR"), OsString::from("/tmp/untrusted-target")),
    ];
    let scrubbed = ScrubbedEnv::from_parent_for_target(snapshot, root);
    assert!(
        scrubbed
            .pairs()
            .iter()
            .any(|(key, value)| { key == "CARGO_TARGET_DIR" && value == ".titania/cache/clippy" })
    );
    assert!(scrubbed.pairs().iter().any(|(key, value)| {
        key == "CARGO_HOME" && value.ends_with("/.titania/hermetic/cargo-home")
    }));
    assert!(scrubbed.pairs().iter().any(|(key, value)| {
        key == "RUSTUP_HOME" && value.ends_with("/.titania/hermetic/rustup-home")
    }));
    assert!(!scrubbed.pairs().iter().any(|(key, value)| {
        value == "/tmp/untrusted-cargo-home"
            || value == "/tmp/untrusted-rustup-home"
            || value == "/tmp/untrusted-target"
            || (key == "CARGO_TARGET_DIR" && value == "/tmp/untrusted-target")
    }));
}

#[cfg(unix)]
#[test]
fn from_parent_drops_non_unicode_key_silently() {
    let bad_key = OsString::from_vec(vec![b'P', b'A', b'T', b'H', 0xFF, 0xFE]);
    let snapshot = vec![
        (bad_key, OsString::from("/tmp/x")),
        (OsString::from("PATH"), OsString::from("/usr/bin")),
        (OsString::from("TERM"), OsString::from("xterm")),
    ];
    let scrubbed = ScrubbedEnv::from_parent(snapshot);
    let kept: Vec<&str> = scrubbed.pairs().iter().map(|(k, _)| k.as_str()).collect();
    assert_eq!(scrubbed.len(), 2, "non-UTF8 key must be silently dropped");
    assert!(kept.contains(&"PATH"));
    assert!(kept.contains(&"TERM"));
}

#[cfg(unix)]
#[test]
fn from_parent_drops_non_unicode_value_silently() {
    let bad_value = OsString::from_vec(vec![0xFF, 0xFE, b'X']);
    let snapshot = vec![
        (OsString::from("PATH"), bad_value),
        (OsString::from("TERM"), OsString::from("xterm")),
    ];
    let scrubbed = ScrubbedEnv::from_parent(snapshot);
    assert_eq!(scrubbed.len(), 1);
    assert_eq!(scrubbed.pairs()[0].0, "TERM");
}

#[test]
fn apply_to_carries_surviving_pairs() {
    let mut cmd = std::process::Command::new(std::env::current_exe().expect("test executable"));
    let scrubbed = ScrubbedEnv::from_iter(pairs(&[
        ("PATH", "/usr/bin"),
        ("RUSTFLAGS", "--harmful"),
        ("FOO", "bar"),
    ]));
    scrubbed.apply_to(&mut cmd);
    let collected_keys: Vec<String> = cmd
        .get_envs()
        .filter_map(|(key, value)| value.map(|_| key.to_string_lossy().into_owned()))
        .collect();
    assert!(collected_keys.iter().any(|k| k == "PATH"));
    assert!(!collected_keys.iter().any(|k| k == "RUSTFLAGS"));
    assert!(!collected_keys.iter().any(|k| k == "FOO"));
}
