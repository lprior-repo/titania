use super::CargoLane;

const FMT_ARGS: &[&str] = &["fmt", "--all", "--check"];
const COMPILE_ARGS: &[&str] =
    &["check", "--workspace", "--all-targets", "--all-features", "--frozen"];
const CLIPPY_ARGS: &[&str] = &[
    "clippy",
    "--workspace",
    "--lib",
    "--bins",
    "--examples",
    "--all-features",
    "--frozen",
    "--message-format=json",
    "--",
    "-D",
    "warnings",
    "-D",
    "unsafe_code",
    "-D",
    "clippy::all",
    "-D",
    "clippy::cargo",
    "-D",
    "clippy::pedantic",
    "-D",
    "clippy::nursery",
    "-D",
    "clippy::unwrap_used",
    "-D",
    "clippy::expect_used",
    "-D",
    "clippy::unwrap_in_result",
    "-D",
    "clippy::panic",
    "-D",
    "clippy::panic_in_result_fn",
    "-D",
    "clippy::todo",
    "-D",
    "clippy::unimplemented",
    "-D",
    "clippy::unreachable",
    "-D",
    "clippy::dbg_macro",
    "-D",
    "clippy::print_stdout",
    "-D",
    "clippy::print_stderr",
    "-D",
    "clippy::indexing_slicing",
    "-D",
    "clippy::string_slice",
    "-D",
    "clippy::get_unwrap",
    "-D",
    "clippy::arithmetic_side_effects",
    "-D",
    "clippy::as_conversions",
    "-D",
    "clippy::integer_division",
    "-D",
    "clippy::integer_division_remainder_used",
    "-D",
    "clippy::let_underscore_must_use",
    "-D",
    "clippy::await_holding_lock",
    "-D",
    "clippy::future_not_send",
    "-D",
    "clippy::large_futures",
    "-D",
    "clippy::allow_attributes",
    "-D",
    "clippy::allow_attributes_without_reason",
    "-D",
    "clippy::disallowed_methods",
    "-D",
    "clippy::disallowed_macros",
    "-D",
    "clippy::disallowed_types",
    "-D",
    "clippy::disallowed_fields",
];
const TEST_ARGS: &[&str] = &["test", "--workspace", "--frozen", "--", "--test-threads=1"];
const BUILD_ARGS: &[&str] = &["build", "--workspace", "--release", "--frozen"];
const FMT_VERSION_ARGS: &[&str] = &["fmt", "--version"];
const CARGO_VERSION_ARGS: &[&str] = &["--version"];

pub(super) const fn args_for_lane(lane: CargoLane) -> &'static [&'static str] {
    match lane {
        CargoLane::Fmt => FMT_ARGS,
        CargoLane::Compile => COMPILE_ARGS,
        CargoLane::Clippy => CLIPPY_ARGS,
        CargoLane::Test => TEST_ARGS,
        CargoLane::Build => BUILD_ARGS,
    }
}

pub(super) const fn version_args(lane: CargoLane) -> &'static [&'static str] {
    match lane {
        CargoLane::Fmt => FMT_VERSION_ARGS,
        CargoLane::Compile | CargoLane::Clippy | CargoLane::Test | CargoLane::Build => {
            CARGO_VERSION_ARGS
        }
    }
}

pub(super) const fn core_lane(lane: CargoLane) -> titania_core::Lane {
    match lane {
        CargoLane::Fmt => titania_core::Lane::Fmt,
        CargoLane::Compile => titania_core::Lane::Compile,
        CargoLane::Clippy => titania_core::Lane::Clippy,
        CargoLane::Test => titania_core::Lane::Test,
        CargoLane::Build => titania_core::Lane::Build,
    }
}

#[cfg(test)]
mod tests {
    use super::{CargoLane, args_for_lane, core_lane, version_args};

    fn assert_deny_pair(argv: &[&str], lint: &str) {
        let marker = argv.iter().enumerate().find_map(|(idx, arg)| {
            if *arg == lint && idx > 0 { argv.get(idx - 1).copied() } else { None }
        });
        assert!(marker == Some("-D"), "lint `{lint}` must be preceded by `-D`; got {:?}", argv,);
    }

    #[test]
    fn compile_args_match_v1_section_9_10_step_1() {
        let argv = args_for_lane(CargoLane::Compile);
        assert_eq!(argv, &["check", "--workspace", "--all-targets", "--all-features", "--frozen"],);
        assert_eq!(argv.len(), 5);
        assert!(argv.contains(&"--all-targets"));
        assert!(argv.contains(&"--all-features"));
        assert!(argv.contains(&"--frozen"));
    }

    #[test]
    fn clippy_args_match_v1_section_9_2_exactly() {
        let argv = args_for_lane(CargoLane::Clippy);
        assert_eq!(
            argv,
            &[
                "clippy",
                "--workspace",
                "--lib",
                "--bins",
                "--examples",
                "--all-features",
                "--frozen",
                "--message-format=json",
                "--",
                "-D",
                "warnings",
                "-D",
                "unsafe_code",
                "-D",
                "clippy::all",
                "-D",
                "clippy::cargo",
                "-D",
                "clippy::pedantic",
                "-D",
                "clippy::nursery",
                "-D",
                "clippy::unwrap_used",
                "-D",
                "clippy::expect_used",
                "-D",
                "clippy::unwrap_in_result",
                "-D",
                "clippy::panic",
                "-D",
                "clippy::panic_in_result_fn",
                "-D",
                "clippy::todo",
                "-D",
                "clippy::unimplemented",
                "-D",
                "clippy::unreachable",
                "-D",
                "clippy::dbg_macro",
                "-D",
                "clippy::print_stdout",
                "-D",
                "clippy::print_stderr",
                "-D",
                "clippy::indexing_slicing",
                "-D",
                "clippy::string_slice",
                "-D",
                "clippy::get_unwrap",
                "-D",
                "clippy::arithmetic_side_effects",
                "-D",
                "clippy::as_conversions",
                "-D",
                "clippy::integer_division",
                "-D",
                "clippy::integer_division_remainder_used",
                "-D",
                "clippy::let_underscore_must_use",
                "-D",
                "clippy::await_holding_lock",
                "-D",
                "clippy::future_not_send",
                "-D",
                "clippy::large_futures",
                "-D",
                "clippy::allow_attributes",
                "-D",
                "clippy::allow_attributes_without_reason",
                "-D",
                "clippy::disallowed_methods",
                "-D",
                "clippy::disallowed_macros",
                "-D",
                "clippy::disallowed_types",
                "-D",
                "clippy::disallowed_fields",
            ],
        );
    }

    #[test]
    fn clippy_args_deny_section_9_2_group_lints() {
        let argv = args_for_lane(CargoLane::Clippy);
        for lint in ["clippy::all", "clippy::cargo", "clippy::pedantic", "clippy::nursery"] {
            assert_deny_pair(argv, lint);
        }
    }

    #[test]
    fn clippy_args_deny_section_9_2_unwrap_expect_panic_families() {
        let argv = args_for_lane(CargoLane::Clippy);
        for lint in [
            "clippy::unwrap_used",
            "clippy::expect_used",
            "clippy::unwrap_in_result",
            "clippy::panic",
            "clippy::panic_in_result_fn",
        ] {
            assert_deny_pair(argv, lint);
        }
    }

    #[test]
    fn clippy_args_deny_section_9_2_todo_macro_and_indexing_families() {
        let argv = args_for_lane(CargoLane::Clippy);
        for lint in [
            "clippy::todo",
            "clippy::unimplemented",
            "clippy::unreachable",
            "clippy::dbg_macro",
            "clippy::print_stdout",
            "clippy::print_stderr",
            "clippy::indexing_slicing",
            "clippy::string_slice",
            "clippy::get_unwrap",
        ] {
            assert_deny_pair(argv, lint);
        }
    }

    #[test]
    fn clippy_args_deny_section_9_2_arithmetic_and_cast_families() {
        let argv = args_for_lane(CargoLane::Clippy);
        for lint in [
            "clippy::arithmetic_side_effects",
            "clippy::as_conversions",
            "clippy::integer_division",
            "clippy::integer_division_remainder_used",
        ] {
            assert_deny_pair(argv, lint);
        }
    }

    #[test]
    fn clippy_args_deny_section_9_2_lock_future_families() {
        let argv = args_for_lane(CargoLane::Clippy);
        for lint in [
            "clippy::let_underscore_must_use",
            "clippy::await_holding_lock",
            "clippy::future_not_send",
            "clippy::large_futures",
        ] {
            assert_deny_pair(argv, lint);
        }
    }

    #[test]
    fn clippy_args_deny_section_9_2_allow_attribute_family() {
        let argv = args_for_lane(CargoLane::Clippy);
        for lint in ["clippy::allow_attributes", "clippy::allow_attributes_without_reason"] {
            assert_deny_pair(argv, lint);
        }
    }

    #[test]
    fn clippy_args_deny_section_9_2_disallowed_family() {
        let argv = args_for_lane(CargoLane::Clippy);
        for lint in [
            "clippy::disallowed_methods",
            "clippy::disallowed_macros",
            "clippy::disallowed_types",
            "clippy::disallowed_fields",
        ] {
            assert_deny_pair(argv, lint);
        }
    }

    #[test]
    fn clippy_args_emit_warnings_and_unsafe_code_denial() {
        let argv = args_for_lane(CargoLane::Clippy);
        assert_deny_pair(argv, "warnings");
        assert_deny_pair(argv, "unsafe_code");
    }

    #[test]
    fn clippy_args_have_exact_section_9_2_length() {
        let argv = args_for_lane(CargoLane::Clippy);
        const DENY_PAIRS: usize = 34;
        const CARGO_PREFIX: usize = 9;
        assert_eq!(argv.len(), CARGO_PREFIX + 2 * DENY_PAIRS);
    }

    #[test]
    fn no_lane_argv_carries_cap_lints() {
        for lane in [
            CargoLane::Fmt,
            CargoLane::Compile,
            CargoLane::Clippy,
            CargoLane::Test,
            CargoLane::Build,
        ] {
            let ceiling = args_for_lane(lane)
                .iter()
                .find(|arg| crate::policy_scan::invocation::is_cap_lints_flag(arg));
            assert!(
                ceiling.is_none(),
                "static argv for {lane:?} must not cap lints; found {ceiling:?}",
            );
        }
    }

    #[test]
    fn args_for_lane_is_deterministic_and_bounded() {
        let compile_a = args_for_lane(CargoLane::Compile);
        let compile_b = args_for_lane(CargoLane::Compile);
        assert_eq!(compile_a.as_ptr(), compile_b.as_ptr());
        assert_eq!(compile_a.len(), compile_b.len());

        let clippy_a = args_for_lane(CargoLane::Clippy);
        let clippy_b = args_for_lane(CargoLane::Clippy);
        assert_eq!(clippy_a.as_ptr(), clippy_b.as_ptr());
        assert!(clippy_a.len() <= 96, "clippy argv must remain bounded");

        let fmt_a = args_for_lane(CargoLane::Fmt);
        let test_a = args_for_lane(CargoLane::Test);
        let build_a = args_for_lane(CargoLane::Build);
        assert!(fmt_a.len() <= 8);
        assert!(test_a.len() <= 8);
        assert!(build_a.len() <= 8);
    }

    #[test]
    fn version_args_route_fmt_separately_from_other_lanes() {
        assert_eq!(version_args(CargoLane::Fmt), &["fmt", "--version"]);
        assert_eq!(version_args(CargoLane::Compile), &["--version"]);
        assert_eq!(version_args(CargoLane::Clippy), &["--version"]);
        assert_eq!(version_args(CargoLane::Test), &["--version"]);
        assert_eq!(version_args(CargoLane::Build), &["--version"]);
    }

    #[test]
    fn core_lane_round_trips_through_cargo_lane() {
        for lane in [
            CargoLane::Fmt,
            CargoLane::Compile,
            CargoLane::Clippy,
            CargoLane::Test,
            CargoLane::Build,
        ] {
            let core = core_lane(lane);
            let expected = format!("{:?}", lane);
            let actual = format!("{core:?}");
            assert!(
                actual.ends_with(&expected),
                "core_lane({lane:?}) -> {actual:?} must end with {expected:?}",
            );
        }
    }
}
