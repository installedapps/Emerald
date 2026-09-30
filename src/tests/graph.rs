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
