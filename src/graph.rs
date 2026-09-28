use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::parser::extract_references;

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
        tracing::debug!(root = %root.display(), files = files.len(), "building link graph");
        let mut names = BTreeSet::new();
        for file in files {
            names.insert(file.clone());
        }
        let mut edges = Vec::new();
        let mut errors = Vec::new();

        for source in files {
            let text = active
                .filter(|(path, _)| *path == source.as_path())
                .map(|(_, text)| text.to_string())
                .or_else(|| std::fs::read_to_string(source).ok());
            let Some(text) = text else {
                let message = format!(
                    "Could not read {}; note has not been made",
                    source.display()
                );
                tracing::warn!(file = %source.display(), "{message}");
                errors.push(message);
                continue;
            };
            for reference in extract_references(&text) {
                let target = reference.file_target();
                if !target.ends_with(".adoc") || !valid_target(&target) {
                    errors.push(format!(
                        "Invalid link target '{target}'; note has not been made"
                    ));
                    continue;
                }
                let target_path = root.join(&target);
                // Workspace membership creates a node, but only an explicit
                // link in the source note creates an edge.
                let edge = GraphEdge {
                    source: source.clone(),
                    target: target_path.clone(),
                };
                if !edges.contains(&edge) {
                    edges.push(edge);
                }
                names.insert(target_path);
            }
        }

        Self {
            nodes: names.into_iter().collect(),
            edges,
            errors,
        }
    }
}

fn valid_target(target: &str) -> bool {
    let path = Path::new(target);
    !path.is_absolute() && path.components().count() == 1 && !target.contains(['\\', ':'])
}

pub fn target_file_name(path: &Path) -> Option<&str> {
    path.file_name()?
        .to_str()
        .filter(|name| name.ends_with(".adoc"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_adoc_edges_and_missing_nodes_from_native_links() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("one.adoc");
        let target = dir.path().join("two.adoc");
        std::fs::write(&source, "xref:two.adoc[Two] and [[three]]").unwrap();
        std::fs::write(&target, "").unwrap();

        let graph = LinkGraph::build(dir.path(), &[source.clone(), target.clone()], None);
        assert_eq!(graph.edges.len(), 2);
        assert!(graph.nodes.contains(&dir.path().join("three.adoc")));
    }

    #[test]
    fn rejects_non_adoc_and_unsafe_targets_without_creating_nodes() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("one.adoc");
        std::fs::write(&source, "xref:bad.txt[X] xref:../escape.adoc[Y]").unwrap();

        let graph = LinkGraph::build(dir.path(), std::slice::from_ref(&source), None);
        assert!(graph.edges.is_empty());
        assert_eq!(graph.nodes, vec![source]);
        assert_eq!(graph.errors.len(), 2);
    }

    #[test]
    fn keeps_unlinked_workspace_notes_as_standalone_nodes() {
        let dir = tempfile::tempdir().unwrap();
        let one = dir.path().join("one.adoc");
        let two = dir.path().join("two.adoc");
        let three = dir.path().join("three.adoc");
        std::fs::write(&one, "xref:two.adoc[Two]").unwrap();
        std::fs::write(&two, "").unwrap();
        std::fs::write(&three, "").unwrap();

        let graph = LinkGraph::build(dir.path(), &[one.clone(), two.clone(), three.clone()], None);

        assert_eq!(
            graph.edges,
            vec![GraphEdge {
                source: one,
                target: two
            }]
        );
        assert!(graph.nodes.contains(&three));
    }
}
