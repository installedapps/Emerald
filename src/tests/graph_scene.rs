use super::*;
use emerald::graph::GraphEdge;

fn linked_graph() -> LinkGraph {
    LinkGraph {
        nodes: ["one.adoc", "two.adoc", "three.adoc", "orphan.adoc"]
            .map(Into::into)
            .to_vec(),
        edges: [("one.adoc", "two.adoc"), ("two.adoc", "three.adoc")]
            .map(|(source, target)| GraphEdge {
                source: source.into(),
                target: target.into(),
            })
            .to_vec(),
        errors: vec![],
    }
}

#[test]
fn highlights_incoming_and_outgoing_neighbors_but_not_unrelated_notes() {
    let graph = linked_graph();
    for (hovered, expected) in [
        (0, vec![true, true, false, false]),
        (1, vec![true, true, true, false]),
        (3, vec![false, false, false, true]),
    ] {
        let scene = GraphScene::new(&graph, Some(hovered), 1.0);
        assert_eq!(
            scene
                .nodes
                .iter()
                .map(|node| node.highlighted)
                .collect::<Vec<_>>(),
            expected
        );
    }
    let scene = GraphScene::new(&graph, Some(0), 1.0);
    assert!(scene.edges[0].highlighted);
    assert!(!scene.edges[1].highlighted);
}

#[test]
fn connections_share_node_coordinates_at_every_zoom() {
    let graph = linked_graph();
    for zoom in [0.45, 1.0, 2.5] {
        let scene = GraphScene::new(&graph, None, zoom);
        assert_eq!(scene.edges.len(), graph.edges.len());
        for (line, edge) in scene.edges.iter().zip(&graph.edges) {
            assert_eq!(graph.nodes[line.source], edge.source);
            assert_eq!(graph.nodes[line.target], edge.target);
            assert_ne!(
                scene.nodes[line.source].center,
                scene.nodes[line.target].center
            );
        }
        assert_eq!(scene.nodes[0].center, (600.0 * zoom, 380.0 * zoom));
        assert_eq!(scene.nodes[1].center, (780.0 * zoom, 380.0 * zoom));
        assert!(scene.nodes[1].radius > scene.nodes[0].radius);
        assert!(scene.nodes.iter().all(|node| node.highlighted));
    }
}

#[test]
fn graph_nodes_are_spread_across_concentric_rings() {
    let positions: Vec<_> = (0..25).map(graph_position).collect();
    for (index, position) in positions.iter().enumerate() {
        for other in positions.iter().skip(index + 1) {
            assert!((position.0 - other.0).hypot(position.1 - other.1) > 50.0);
        }
    }
}
