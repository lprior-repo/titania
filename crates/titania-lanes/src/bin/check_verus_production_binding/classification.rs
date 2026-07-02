fn has_proof_fn(text: &str) -> bool {
    find_subslice(text, 0, "proof fn").is_some()
}

fn classify(rel: &str, text: &str) -> ProofScan {
    if is_fixture_smoke(rel, text) {
        return ProofScan::NotApplicable(NotApplicableReason::FixtureSmoke);
    }
    match (first_path_target(text).as_deref(), has_assume_specification(text)) {
        (Some(p), true) if p.contains("crates/") && !p.contains("proof_kernels/") => {
            ProofScan::Binding(Binding::Strong)
        }
        (Some(p), true) if p.contains("production_inner/") || p.contains("proof_kernels/") => {
            ProofScan::Binding(Binding::Weak)
        }
        (Some(_), true) => ProofScan::Binding(Binding::Weak),
        (Some(_), false) | (None, true | false) => ProofScan::Vacuum,
    }
}

fn is_fixture_smoke(rel: &str, text: &str) -> bool {
    rel == FORMAL_SETUP_SMOKE_FILE
        && text.contains(FIXTURE_SMOKE_MARKER)
        && find_subslice(text, 0, "proof fn formal_setup_smoke").is_some()
}

fn first_path_target(text: &str) -> Option<String> {
    text.match_indices("#[path = \"")
        .next()
        .and_then(|(start, needle)| path_target_after(text, start.saturating_add(needle.len())))
}

fn path_target_after(text: &str, start: usize) -> Option<String> {
    text.get(start..).map(|rest| rest.chars().take_while(|c| *c != '"').collect())
}

fn has_assume_specification(text: &str) -> bool {
    text.contains("assume_specification[")
}

fn find_subslice(text: &str, start: usize, needle: &str) -> Option<usize> {
    let rest = text.get(start..)?;
    rest.find(needle).map(|off| start.saturating_add(off))
}
