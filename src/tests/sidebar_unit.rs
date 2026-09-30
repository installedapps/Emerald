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
fn sidebar_width_drag_direction_and_limits_are_consistent() {
    use sidebar_resize::resized_width as resize;
    assert_eq!(resize(280.0, 100.0, 140.0, false), 320.0);
    assert_eq!(resize(260.0, 300.0, 240.0, true), 320.0);
    assert_eq!(resize(200.0, 0.0, -100.0, false), 180.0);
    assert_eq!(resize(500.0, 0.0, -100.0, true), 520.0);
}

#[test]
fn sidebar_icons_use_the_current_theme_accent() {
    let theme = emerald::theme::ThemeId::TokyoNight.theme();
    let svg = sidebar_controls::themed_sidebar_icon(theme);
    assert!(svg.contains(&format!("#{:06x}", theme.accent)));
    assert!(!svg.contains("#000000"));
}
