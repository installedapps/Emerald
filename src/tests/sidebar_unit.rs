use super::*;
use std::path::PathBuf;

#[test]
fn outgoing_links_only_include_unique_targets_from_the_active_note() {
    let active = PathBuf::from("notes/active.adoc");
    let other = PathBuf::from("notes/other.adoc");
    let first = PathBuf::from("notes/first.adoc");
    let second = PathBuf::from("notes/second.adoc");
    let graph = emerald::graph::LinkGraph {
        nodes: vec![],
        edges: vec![
            emerald::graph::GraphEdge {
                source: active.clone(),
                target: first.clone(),
            },
            emerald::graph::GraphEdge {
                source: active.clone(),
                target: first.clone(),
            },
            emerald::graph::GraphEdge {
                source: active.clone(),
                target: second.clone(),
            },
            emerald::graph::GraphEdge {
                source: other,
                target: active.clone(),
            },
        ],
        errors: vec![],
    };
    assert_eq!(
        sidebar_links::outgoing_targets(&graph, &active),
        vec![first, second]
    );
}

#[test]
fn linked_note_labels_and_routes_use_the_asciidoc_filename() {
    let path = PathBuf::from("notes/design_notes.adoc");
    assert_eq!(sidebar_links::note_label(&path), "design notes");
    assert_eq!(
        sidebar_links::note_target(&path).as_deref(),
        Some("design_notes.adoc")
    );
    assert_eq!(
        sidebar_links::note_target(std::path::Path::new("notes/image.png")),
        None
    );
}

#[test]
fn forward_links_keep_each_active_note_link_occurrence() {
    let dir = tempfile::tempdir().unwrap();
    let active = dir.path().join("active.adoc");
    let target = dir.path().join("target.adoc");
    let other = dir.path().join("other.adoc");
    std::fs::write(
        &active,
        "xref:target.adoc[First] and [[target.adoc|Second]]",
    )
    .unwrap();
    std::fs::write(&target, "Target note").unwrap();
    std::fs::write(&other, "xref:active.adoc[Back to active]").unwrap();
    let files = vec![active.clone(), target.clone(), other];
    let index = emerald::index::WorkspaceIndex::build(dir.path(), &files, None);

    let outgoing = index.forward_links_from(&active).collect::<Vec<_>>();
    assert_eq!(outgoing.len(), 2);
    assert_eq!(outgoing[0].label, "First");
    assert_eq!(outgoing[1].label, "Second");
    assert_eq!(
        outgoing[0].target,
        emerald::index::ResolvedTarget::ExistingNote(target.clone())
    );
    assert_eq!(
        &index.note(&active).unwrap().text[outgoing[0].source_range.clone()],
        "xref:target.adoc[First]"
    );
}

#[test]
fn link_panel_direction_toggle_switches_both_ways() {
    use sidebar_links::LinkPanelDirection;

    assert_eq!(
        LinkPanelDirection::Backlinks.toggled(),
        LinkPanelDirection::ForwardLinks
    );
    assert_eq!(
        LinkPanelDirection::ForwardLinks.toggled(),
        LinkPanelDirection::Backlinks
    );
}

#[test]
fn sidebar_width_drag_direction_and_limits_are_consistent() {
    use sidebar_resize::resized_width as resize;
    assert_eq!(resize(280.0, 100.0, 140.0, false), 320.0);
    assert_eq!(resize(260.0, 300.0, 240.0, true), 320.0);
    assert_eq!(resize(200.0, 0.0, -100.0, false), 180.0);
    assert_eq!(resize(500.0, 0.0, -100.0, true), 520.0);
}
