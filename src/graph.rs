use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphEdge {
    pub source: PathBuf,
    pub target: PathBuf,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LinkGraph {
    pub nodes: Vec<PathBuf>,
    pub edges: Vec<GraphEdge>,
    pub errors: Vec<String>,
}

impl LinkGraph {
    pub fn build(root: &Path, files: &[PathBuf], active: Option<(&Path, &str)>) -> Self {
        crate::index::WorkspaceIndex::build(root, files, active)
            .graph()
            .clone()
    }
}

pub fn target_file_name(path: &Path) -> Option<&str> {
    path.file_name()?
        .to_str()
        .filter(|name| name.ends_with(".adoc"))
}

#[cfg(test)]
#[path = "tests/graph.rs"]
mod tests;
