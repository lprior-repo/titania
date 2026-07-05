use titania_core::{LaneFailure, LaneOutcome, TargetProject};

use crate::{
    CommandOutput, LaneError,
    dylint_lane::{DylintProbe, probe_dylint_toolchain},
};

pub(super) fn outcome(target: &TargetProject) -> LaneOutcome {
    match probe_dylint_toolchain() {
        DylintProbe::Infra(failure, _) => return LaneOutcome::Failed(failure),
        DylintProbe::Ready => {}
    }

    let output = match super::command_output(target, "cargo", &["dylint", "--workspace", "--all"]) {
        Ok(output) => output,
        Err(error) => return failure_outcome(&error),
    };

    if output.success() {
        return super::run_lane_outcome::clean_outcome_unchecked(
            titania_core::Lane::Dylint,
            "cargo-dylint",
            output.stdout(),
        );
    }
    LaneOutcome::Failed(LaneFailure::Suspicious {
        tool: String::from("cargo-dylint"),
        evidence: failure_evidence(&output),
    })
}

fn failure_evidence(output: &CommandOutput) -> String {
    output
        .stderr_str()
        .map_or_else(|_| String::from("<non-UTF-8>"), |stderr| stderr_or_status(stderr, output))
}

fn stderr_or_status(stderr: &str, output: &CommandOutput) -> String {
    if stderr.is_empty() {
        format!("cargo dylint exited with code {:?}", output.status().code())
    } else {
        stderr.to_owned()
    }
}

fn failure_outcome(error: &LaneError) -> LaneOutcome {
    LaneOutcome::Failed(LaneFailure::Infra {
        tool: String::from("cargo-dylint"),
        reason: error.to_string(),
    })
}
