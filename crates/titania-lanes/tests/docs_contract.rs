fn workspace_root() -> std::path::PathBuf {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
    std::path::PathBuf::from(manifest).join("../../")
}

fn read_ws_file(name: &str) -> String {
    let path = workspace_root().join(name);
    std::fs::read_to_string(&path).unwrap_or_else(|_| {
        panic!("read {name}: {path:?} (run from repo root with CARGO_MANIFEST_DIR set)")
    })
}

const FORBIDDEN: &[&str] = &[
    "titania init",
    "titania doctor",
    "titania ci --scope edit",
    "titania ci --scope prepush",
    "titania ci --scope full",
    "vb-fmt-0012",
];

const FORMAL_PROOF_CLAIMS: &[&str] = &[
    "v1 proves panic-freedom",
    "proves functional correctness",
    "proves panic freedom",
    "functional correctness proof",
];

const REQUIRED_V1_COMMANDS: &[&str] = &[
    "cargo generate titania/template",
    "titania-check doctor",
    "titania-check --scope edit",
    "titania-check --scope prepush",
    "titania-check --scope release",
];

#[test]
fn v1_docs_contract_no_forbidden_strings() {
    let readme = read_ws_file("README.md");
    let vision = read_ws_file("VISION.md");
    let _combined = format!("{readme}\n{vision}");

    let mut violations: Vec<(&str, &str)> = Vec::new();

    for text in [(&readme, "README.md"), (&vision, "VISION.md")] {
        for pattern in FORBIDDEN {
            if text.0.contains(*pattern) {
                violations.push((pattern, text.1));
            }
        }
        for pattern in FORMAL_PROOF_CLAIMS {
            if text.0.to_lowercase().contains(*pattern) {
                violations.push((pattern, text.1));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Forbidden strings found in v1 public docs:\n{}",
        violations.iter().map(|(s, f)| format!("  - {s:?} in {f}")).collect::<Vec<_>>().join("\n"),
    );
}

#[test]
fn v1_docs_contract_required_commands() {
    let readme = read_ws_file("README.md");
    let vision = read_ws_file("VISION.md");

    let mut missing: Vec<&str> = Vec::new();

    for cmd in REQUIRED_V1_COMMANDS {
        if !readme.contains(*cmd) && !vision.contains(*cmd) {
            missing.push(cmd);
        }
    }

    assert!(
        missing.is_empty(),
        "Required v1 commands not found in README.md or VISION.md:\n{}",
        missing.iter().map(|c| format!("  - {c}")).collect::<Vec<_>>().join("\n"),
    );
}

#[test]
fn v1_docs_contract_vision_links_v1spec() {
    let vision = read_ws_file("VISION.md");

    assert!(
        vision.contains("v1-spec.md"),
        "VISION.md must link to v1-spec.md as the concrete buildable v1 contract",
    );
}

#[test]
fn v1_docs_contract_scopes_match_gatescope() {
    let readme = read_ws_file("README.md");

    let scopes_in_readme: Vec<&str> = ["edit", "prepush", "release", "full", "deep"]
        .into_iter()
        .filter(|s| readme.contains(&format!("--scope {s}")))
        .collect();

    let out_of_scope: Vec<&str> =
        scopes_in_readme.into_iter().filter(|s| *s == "full" || *s == "deep").collect();

    assert!(
        out_of_scope.is_empty(),
        "README.md must not advertise v1.5+ scopes as v1:\n{}",
        out_of_scope.iter().map(|s| format!("  - --scope {s}")).collect::<Vec<_>>().join("\n"),
    );
}

#[test]
fn v1_docs_contract_sync() {
    v1_docs_contract_no_forbidden_strings();
    v1_docs_contract_required_commands();
    v1_docs_contract_vision_links_v1spec();
    v1_docs_contract_scopes_match_gatescope();
}
