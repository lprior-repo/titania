use std::{
    collections::BTreeSet,
    ffi::{OsStr, OsString},
    path::Path,
};

const DEFAULT_DENY_KEYS: &[&str] = &[
    "RUSTFLAGS",
    "RUSTC_WRAPPER",
    "RUSTC_WORKSPACE_WRAPPER",
    "CARGO_ENCODED_RUSTFLAGS",
    "CARGO_INCREMENTAL",
    "RUSTC_BOOTSTRAP",
    "CARGO_DYLINT_LINTS",
    "DYLINT_DRIVER",
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    "RUSTUP_TOOLCHAIN",
    "CARGO_HOME",
    "RUSTUP_HOME",
    "TITANIA_DISABLE_LANES",
    "TITANIA_FORCE_CLEAN",
];

const DEFAULT_DENY_PREFIXES: &[&str] = &["CARGO_", "BOXED_"];

const DEFAULT_ALLOW_KEYS: &[&str] = &[
    "PATH",
    "LANG",
    "LC_ALL",
    "LC_CTYPE",
    "LC_MESSAGES",
    "LC_COLLATE",
    "LC_NUMERIC",
    "LC_TIME",
    "TERM",
    "http_proxy",
    "https_proxy",
    "no_proxy",
    "HTTP_PROXY",
    "HTTPS_PROXY",
    "NO_PROXY",
    "TZ",
    "HOME",
    "USERPROFILE",
    "HOMEDRIVE",
    "HOMEPATH",
    "SystemRoot",
    "APPDATA",
    "LOCALAPPDATA",
    "USER",
    "LOGNAME",
    "XDG_CONFIG_HOME",
    "XDG_CACHE_HOME",
    "XDG_DATA_HOME",
    "SSL_CERT_FILE",
    "SSL_CERT_DIR",
    "CURL_CA_BUNDLE",
    "SSH_AUTH_SOCK",
    "SSH_AGENT_PID",
];

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

    fn keeps(&self, key: &str) -> bool {
        !key.contains('\0')
            && !self.deny_keys.iter().any(|denied| names_equal(denied, key))
            && !self.deny_prefixes.iter().any(|prefix| starts_with_name(key, prefix))
            && self.allow_keys.iter().any(|allowed| names_equal(allowed, key))
    }
}

#[derive(Debug, Clone)]
pub(super) struct ScrubbedEnv {
    pairs: Vec<(String, String)>,
}

impl ScrubbedEnv {
    #[must_use]
    pub(super) fn from_parent(vars_os: impl IntoIterator<Item = (OsString, OsString)>) -> Self {
        let policy = ScrubPolicy::new(DEFAULT_ALLOW_KEYS, DEFAULT_DENY_KEYS, DEFAULT_DENY_PREFIXES);
        Self::from_os(vars_os, &policy)
    }

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

    pub(super) fn apply_to(self, cmd: &mut std::process::Command) {
        apply_pairs(cmd, &self.pairs);
    }

    #[cfg(test)]
    #[must_use]
    fn pairs(&self) -> &[(String, String)] {
        &self.pairs
    }

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

fn apply_pairs(cmd: &mut std::process::Command, pairs: &[(String, String)]) {
    let _ = pairs.iter().fold(&mut *cmd, |cmd, (key, value)| {
        let _ = cmd.env(key, value);
        cmd
    });
}

#[cfg(test)]
#[path = "tests/env_filter.rs"]
mod tests;
