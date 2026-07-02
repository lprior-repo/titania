//! Perf-scope classification for the nightly-features lane.
//!
//! The scope is determined once at the boundary (file path + marker
//! check) and carried as a [`FeatureScope`] enum so the per-feature
//! check matches exhaustively instead of branching on bool flags.

/// Where a file sits relative to the perf-feature opt-in policy.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum FeatureScope {
    /// Default: not perf-scoped and no opt-in marker.
    Normal,
    /// Under `crates/<name>/src/perf/`, `crates/<name>/src/generated/`,
    /// or `benches/`.
    PerfScoped,
    /// Contains the `velvet-allow-perf-nightly-feature` marker.
    MarkerOptIn,
}

/// Boundary signals bundled so the classifier takes a single typed value
/// rather than two independent `bool` flags.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct ScopeSignals {
    pub(super) perf_scoped: bool,
    pub(super) marker_opt_in: bool,
}

/// Classify the scope from the two boundary signals. A perf-scoped path
/// wins over the marker (both permit perf features, but the path is the
/// stronger structural guarantee).
#[must_use]
pub(super) const fn classify_scope(signals: ScopeSignals) -> FeatureScope {
    match (signals.perf_scoped, signals.marker_opt_in) {
        (true, _) => FeatureScope::PerfScoped,
        (false, true) => FeatureScope::MarkerOptIn,
        (false, false) => FeatureScope::Normal,
    }
}

/// A file is perf-scoped under the canonical performance locations.
///
/// Accepted locations are `crates/<name>/src/perf/`,
/// `crates/<name>/src/generated/`, and `benches/`. Path segments are matched
/// explicitly; substring matching would accept unrelated paths.
pub(super) fn is_perf_scoped_path(file: &str) -> bool {
    let normalized = file.replace('\\', "/");
    is_under_crates_perf_or_generated(&normalized) || normalized.starts_with("benches/")
}

fn is_under_crates_perf_or_generated(normalized: &str) -> bool {
    normalized.split('/').collect::<Vec<&str>>().windows(4).any(|w| {
        matches!(
            w,
            ["crates", _, "src", kind] if *kind == "perf" || *kind == "generated"
        )
    })
}

#[cfg(test)]
mod tests {
    use super::is_perf_scoped_path;

    #[test]
    fn perf_scoped_path_recognises_crate_perf_and_generated() {
        assert!(is_perf_scoped_path("crates/foo/src/perf/widget.rs"));
        assert!(is_perf_scoped_path("crates/foo/src/generated/widget.rs"));
        assert!(is_perf_scoped_path("benches/bench.rs"));
        // Outside any perf scope.
        assert!(!is_perf_scoped_path("crates/foo/src/lib.rs"));
    }

    #[test]
    fn perf_scoped_path_rejects_substring_matches() {
        // `mycrates/foo/src/perf/x.rs` must not match (segment check).
        assert!(!is_perf_scoped_path("mycrates/foo/src/perf/x.rs"));
        // A path mentioning perf outside the canonical layout is not scoped.
        assert!(!is_perf_scoped_path("crates/foo/perf/lib.rs"));
        assert!(!is_perf_scoped_path("crates/foo/src/lib/perf.rs"));
    }
}
