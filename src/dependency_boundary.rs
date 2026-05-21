use crate::error::XtaskCommandError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceManifest {
    edges: Vec<DependencyEdge>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct DependencyEdge {
    crate_name: String,
    dependency: String,
}

impl WorkspaceManifest {
    #[must_use]
    pub fn from_edges<I, C, D>(edges: I) -> Self
    where
        I: IntoIterator<Item = (C, D)>,
        C: Into<String>,
        D: Into<String>,
    {
        Self {
            edges: edges.into_iter().map(DependencyEdge::from_pair).collect(),
        }
    }
}

impl DependencyEdge {
    fn from_pair<C, D>((crate_name, dependency): (C, D)) -> Self
    where
        C: Into<String>,
        D: Into<String>,
    {
        Self {
            crate_name: crate_name.into(),
            dependency: dependency.into(),
        }
    }
}

pub fn assert_runtime_dependency_boundary(
    manifest: &WorkspaceManifest,
) -> Result<(), XtaskCommandError> {
    for edge in &manifest.edges {
        if is_runtime_crate(&edge.crate_name) && is_forbidden_runtime_dependency(&edge.dependency) {
            return Err(XtaskCommandError::DependencyBoundaryViolation {
                crate_name: edge.crate_name.clone(),
                dependency: edge.dependency.clone(),
            });
        }
    }
    Ok(())
}

fn is_runtime_crate(crate_name: &str) -> bool {
    matches!(
        crate_name,
        "vb_core" | "vb_runtime" | "vb_storage" | "vb_ipc"
    )
}

fn is_forbidden_runtime_dependency(dependency: &str) -> bool {
    matches!(
        dependency,
        "xtask"
            | "clap"
            | "serde_json"
            | "serde_yaml"
            | "reqwest"
            | "hyper"
            | "toml"
            | "serde-saphyr"
    )
}
