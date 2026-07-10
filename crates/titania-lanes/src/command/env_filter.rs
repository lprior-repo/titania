//! Environment scrubbing for inherited subprocesses.
//!
//! Every lane shells out through [`super::CommandIn`] and several lanes
//! (cargo, cargo-dylint, cargo-deny) intentionally inherit the parent
//! process environment so tools like `cargo` can locate `rustup`,
//! registries, proxy, and certificates. A raw `inherit` pass-through is
//! unsafe: env vars such as `RUSTFLAGS`, `RUSTC_WRAPPER`,
//! `CARGO_ENCODED_RUSTFLAGS`, `RUSTC_BOOTSTRAP`, `LD_PRELOAD`, and
//! `DYLD_*` directly mutate the lane's judgment surface and let a
//! hostile or contaminated parent influence the verdict.
//!
//! This module owns that scrubbing policy. It is a pure-data core: it
//! builds a [`ScrubbedEnv`] snapshot from an arbitrary iterator of
//! `(key, value)` pairs (typically the parent `std::env::vars_os()`
//! snapshot) and applies a deny-by-default filter that keeps only
//! safe-to-inherit keys. [`ScrubbedEnv::apply_to`] then copies the
//! surviving pairs onto a [`std::process::Command`].
//!
//! Non-Unicode inherited variables cannot be matched against the
//! ASCII allow/deny tables (and `std::env::vars()` would panic on
//! them), so they are silently dropped by the `vars_os` constructor.
//!
//! The default deny list is curated for the lane subprocesses that
//! actually exist in this crate (cargo, dylint, deny).

use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    path::Path,
};

/// Env-var keys that must NEVER be inherited as-is by a lane
/// subprocess, even when the user requested inheritance.
///
/// Curated for the actual lanes shipping in `titania-lanes`. Every
/// entry represents a direct bypass vector for cargo/rustc/dylint
/// judgment.
const DEFAULT_DENY_KEYS: &[&str] = &[
    // rustc/cargo compile-flag bypasses:
    "RUSTFLAGS",
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_INCREMENTAL",
    // Tells rustc it can use unstable features; matters for codegen checks.
    "RUSTC_BOOTSTRAP",
    // dylint-specific bypass:
    "CARGO_DYLINT_LINTS",
    "DYLINT_DRIVER",
    // LD/DYLD process-injection bypasses:
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    // Toolchain shim overrides:
    "RUSTUP_TOOLCHAIN",
    "CARGO_HOME",
    "RUSTUP_HOME",
    // Test/CI hermetic-suppression bypasses:
    "TITANIA_DISABLE_LANES",
    "TITANIA_FORCE_CLEAN",
];

/// Prefix-based deny: every key starting with one of these prefixes
/// is dropped.
///
/// Cargo registries, targets, build-jobs, locks, and boxed
/// test-runner env-pass-through all live in the `CARGO_*` / `BOXED_*`
/// namespaces, so prefix-deny is the right tool.
const DEFAULT_DENY_PREFIXES: &[&str] = &["CARGO_", "BOXED_"];

/// The allowlist of keys that ARE safe to inherit. Anything not on
/// this list is dropped. Keeping only the well-known toolchain,
/// terminal, and network keys keeps the surface tiny and reviewable.
const DEFAULT_ALLOW_KEYS: &[&str] = &[
    // Path resolution:
    "PATH",
    // Locale / terminal:
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "LC_MESSAGES",
    "LC_COLLATE",
    "LC_NUMERIC",
    "LC_TIME",
    "TERM",
    // Network / proxy:
    "http_proxy",
    "https_proxy",
    "no_proxy",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    // Timezone:
    "TZ",
    // Home directory lookups:
    "HOME",
    // Windows home/toolchain discovery:
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "SystemRoot",
    "APPDATA",
    "LOCALAPPDATA",
    "USER",
    "LOGNAME",
    // XDG:
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
    "XDG_DATA_HOME",
    // CA bundle TLS roots for cargo registry access:
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "CURL_CA_BUNDLE",
    // ssh agent forwarding so cargo can pull private git deps:
    "SSH_AUTH_SOCK",
    "SSH_AGENT_PID",
];

/// Bundled allow/deny filter tables so per-pair callbacks can carry
/// the policy by reference instead of three slice arguments.
///
/// `Copy` because every field is a borrowed slice; passing the
/// tables around is a borrow, never a move.
#[derive(Copy, Clone, Debug)]
struct ScrubPolicy<'a> {
    allow_keys: &'a [&'a str],
    deny_keys: &'a [&'a str],
    deny_prefixes: &'a [&'a str],
}

impl<'a> ScrubPolicy<'a> {
    const fn new(
        allow_keys: &'a [&'a str],
        deny_keys: &'a [&'a str],
        deny_prefixes: &'a [&'a str],
    ) -> Self {
        Self { allow_keys, deny_keys, deny_prefixes }
    }

    /// Decide whether `key` survives the deny/allow filter.
    fn keeps(&self, key: &str) -> bool {
        !key.contains('\0')
            && !self.deny_keys.iter().any(|denied| names_equal(denied, key))
            && !self.deny_prefixes.iter().any(|prefix| starts_with_name(key, prefix))
            && self.allow_keys.iter().any(|allowed| names_equal(allowed, key))
    }
}

/// A pure-data scrubber that builds the inherited environment for a
/// [`super::CommandIn`] when `inherit_env()` is requested.
///
/// Built from an arbitrary iterator yielding `(key, value)` pairs
/// (typically `std::env::vars_os()`) plus the default deny/allow lists.
/// Apply via [`ScrubbedEnv::apply_to`].
#[derive(Debug, Clone)]
pub(super) struct ScrubbedEnv {
    /// (key, value) pairs that survived the deny+allow filter,
    /// stored as owned `String`s so the snapshot outlives the
    /// parent environment iterator.
    pairs: Vec<(String, String)>,
}

impl ScrubbedEnv {
    /// Scrub the parent process environment (`vars_os()`) into a
    /// snapshot safe to copy onto a subprocess.
    ///
    /// Non-Unicode keys and values are dropped silently: they cannot
    /// be matched against the ASCII deny/allow tables and the
    /// equivalent `std::env::vars()` would panic on them. This
    /// constructor is the safe path used by [`super::CommandIn`].
    ///
    /// Filtering order:
    ///
    /// 1. Drop any key containing a NUL byte (defensive: NULs
    ///    cannot appear in sane env vars).
    /// 2. Drop any key exactly matching `DEFAULT_DENY_KEYS`.
    /// 3. Drop any key with a prefix in `DEFAULT_DENY_PREFIXES`.
    /// 4. Keep only keys matching `DEFAULT_ALLOW_KEYS`.
    /// 5. Drop duplicate keys, keeping first occurrence.
    #[must_use]
    pub(super) fn from_parent(vars_os: impl IntoIterator<Item = (OsString, OsString)>) -> Self {
        let policy = ScrubPolicy::new(DEFAULT_ALLOW_KEYS, DEFAULT_DENY_KEYS, DEFAULT_DENY_PREFIXES);
        Self::from_os(vars_os, &policy)
    }

    /// Scrub inherited variables while preserving the validated per-lane
    /// Cargo target directory and hermetic tool homes supplied by Moon.
    #[must_use]
    pub(super) fn from_parent_for_target(
        vars_os: impl IntoIterator<Item = (OsString, OsString)>,
        target_root: &Path,
    ) -> Self {
        let vars: Vec<(OsString, OsString)> = vars_os.into_iter().collect();
        let mut scrubbed = Self::from_parent(vars.iter().cloned());
        scrubbed.pairs.extend(
            vars.iter().filter_map(|(key, value)| validated_lane_pair(key, value, target_root)),
        );
        scrubbed
    }

    /// Scrub an arbitrary iterator of typed `(K, V)` pairs.
    /// Convenient for unit tests and for callers holding a UTF-8
    /// snapshot of the parent environment.
    #[cfg(test)]
    #[must_use]
    fn from_iter<I, K, V>(parent: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        Self::from_iter_with_policy(
            parent,
            DEFAULT_ALLOW_KEYS,
            DEFAULT_DENY_KEYS,
            DEFAULT_DENY_PREFIXES,
        )
    }

    /// Scrub with caller-supplied allow/deny lists instead of the
    /// defaults. Both `allow_keys` and `deny_keys` accept exact-name
    /// matches; `deny_prefixes` accepts case-sensitive prefixes.
    #[cfg(test)]
    #[must_use]
    fn from_iter_with_policy<I, K, V>(
        parent: I,
        allow_keys: &[&str],
        deny_keys: &[&str],
        deny_prefixes: &[&str],
    ) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        let policy = ScrubPolicy::new(allow_keys, deny_keys, deny_prefixes);
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let pairs = parent
            .into_iter()
            .filter_map(|(k, v)| collect_pair_str(k.into(), v.into(), &policy, &mut seen))
            .collect();
        Self { pairs }
    }

    /// Scrub from `vars_os`-style `(OsString, OsString)` pairs using
    /// caller-supplied allow/deny tables. Non-Unicode variables are
    /// dropped silently to avoid panicking on host environments.
    #[must_use]
    fn from_os(
        vars_os: impl IntoIterator<Item = (OsString, OsString)>,
        policy: &ScrubPolicy<'_>,
    ) -> Self {
        let mut seen: BTreeSet<String> = BTreeSet::new();
        let pairs = vars_os
            .into_iter()
            .filter_map(|(k, v)| collect_pair_os(&k, &v, policy, &mut seen))
            .collect();
        Self { pairs }
    }

    /// Apply the scrubbed snapshot onto `cmd`.
    pub(super) fn apply_to(self, cmd: &mut std::process::Command) {
        apply_pairs(cmd, &self.pairs);
    }

    /// Borrow the surviving pairs for pure tests.
    #[cfg(test)]
    #[must_use]
    fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }

    /// Number of inherited pairs that survive scrubbing.
    #[cfg(test)]
    #[must_use]
    fn len(&self) -> usize {
        self.pairs.len()
    }
}

fn names_equal(left: &str, right: &str) -> bool {
    #[cfg(windows)]
    {
        left.eq_ignore_ascii_case(right)
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn starts_with_name(key: &str, prefix: &str) -> bool {
    #[cfg(windows)]
    {
        key.get(..prefix.len()).is_some_and(|head| head.eq_ignore_ascii_case(prefix))
    }
    #[cfg(not(windows))]
    {
        key.starts_with(prefix)
    }
}

const TRUSTED_TARGET_DIRS: &[&str] = &[
    ".titania/cache/compile",
    ".titania/cache/clippy",
    ".titania/cache/test",
    ".titania/cache/release",
];

fn is_trusted_target_dir(value: &OsStr, target_root: &Path) -> bool {
    let supplied = Path::new(value);
    TRUSTED_TARGET_DIRS.iter().any(|relative| {
        let expected_relative = Path::new(relative);
        supplied == expected_relative || supplied == target_root.join(expected_relative)
    })
}

fn is_trusted_home(value: &str, target_root: &Path, suffix: &str) -> bool {
    Path::new(value) == target_root.join(".titania").join("hermetic").join(suffix)
}

fn key_identity(key: &str) -> String {
    #[cfg(windows)]
    {
        key.to_ascii_uppercase()
    }
    #[cfg(not(windows))]
    {
        key.to_owned()
    }
}

/// Build a `(key, value)` pair only for env vars Moon explicitly
/// validates for the per-lane subprocess (cargo target dir, cargo
/// home, rustup home). All other keys are dropped.
fn validated_lane_pair(key: &OsStr, value: &OsStr, target_root: &Path) -> Option<(String, String)> {
    let key_str = key.to_str()?;
    let value_str = value.to_str()?;
    let trusted = match key_str {
        "CARGO_TARGET_DIR" => is_trusted_target_dir(OsStr::new(value_str), target_root),
        "CARGO_HOME" => is_trusted_home(value_str, target_root, "cargo-home"),
        "RUSTUP_HOME" => is_trusted_home(value_str, target_root, "rustup-home"),
        _ => return None,
    };
    trusted.then(|| (key_str.to_owned(), value_str.to_owned()))
}
#[cfg(test)]
/// String-typed pair collector for [`ScrubbedEnv::from_iter_with_policy`].
fn collect_pair_str(
    key: String,
    value: String,
    policy: &ScrubPolicy<'_>,
    seen: &mut BTreeSet<String>,
) -> Option<(String, String)> {
    if !policy.keeps(&key) {
        return None;
    }
    if !seen.insert(key_identity(&key)) {
        return None;
    }
    Some((key, value))
}

/// OsString-typed pair collector for [`ScrubbedEnv::from_parent`].
///
/// Non-Unicode variables are silently dropped to keep the pure core
/// panic-free on host environments that include non-UTF8 variables.
fn collect_pair_os(
    key: &OsStr,
    value: &OsStr,
    policy: &ScrubPolicy<'_>,
    seen: &mut BTreeSet<String>,
) -> Option<(String, String)> {
    let key_str = key.to_str()?;
    let value_str = value.to_str()?;
    if !policy.keeps(key_str) {
        return None;
    }
    if !seen.insert(key_identity(key_str)) {
        return None;
    }
    Some((key_str.to_owned(), value_str.to_owned()))
}

/// Apply the surviving pairs onto `cmd`.
fn apply_pairs(cmd: &mut std::process::Command, pairs: &[(String, String)]) {
    fn step(
        cmd: &mut std::process::Command,
        (key, value): &(String, String),
    ) -> Result<(), std::convert::Infallible> {
        let _ = cmd.env(key, value);
        Ok(())
    }
    let result: Result<(), std::convert::Infallible> =
        pairs.iter().try_for_each(|pair| step(cmd, pair));
    match result {
        Ok(()) => {}
        Err(unreachable) => match unreachable {},
    }
}

#[cfg(test)]
#[path = "tests/env_filter.rs"]
mod tests;
