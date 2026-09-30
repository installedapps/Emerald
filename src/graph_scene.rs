use std::collections::BTreeMap;

use emerald::graph::LinkGraph;

pub(super) const GRAPH_WIDTH: f32 = 1200.0;
pub(super) const GRAPH_HEIGHT: f32 = 760.0;

pub(super) struct GraphNode {
    pub center: (f32, f32),
    pub radius: f32,
    pub highlighted: bool,
}

pub(super) struct GraphLine {
    pub source: usize,
    pub target: usize,
    pub highlighted: bool,
}

/// One geometry snapshot for both edge painting and interactive nodes.
/// Coordinates include zoom; panning belongs to their shared parent layer.
pub(super) struct GraphScene {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphLine>,
}

impl GraphScene {
    pub fn new(graph: &LinkGraph, hovered: Option<usize>, zoom: f32) -> Self {
        let indices: BTreeMap<_, _> = graph
            .nodes
            .iter()
            .enumerate()
            .map(|(index, path)| (path, index))
            .collect();
        let hovered = hovered.filter(|index| *index < graph.nodes.len());
        let mut degrees = vec![0; graph.nodes.len()];
        let mut highlighted = vec![hovered.is_none(); graph.nodes.len()];
        if let Some(index) = hovered {
            highlighted[index] = true;
        }
        let mut edges = Vec::with_capacity(graph.edges.len());
        for edge in &graph.edges {
            let (Some(&source), Some(&target)) =
                (indices.get(&edge.source), indices.get(&edge.target))
            else {
                continue;
            };
            let connected = hovered == Some(source) || hovered == Some(target);
            degrees[source] += 1;
            degrees[target] += 1;
            if connected {
                highlighted[source] = true;
                highlighted[target] = true;
            }
            edges.push(GraphLine {
                source,
                target,
                highlighted: hovered.is_none() || connected,
            });
        }
        let nodes = degrees
            .into_iter()
            .enumerate()
            .map(|(index, degree)| {
                let (x, y) = graph_position(index);
                GraphNode {
                    center: (x * zoom, y * zoom),
                    radius: (5.0 + degree as f32 * 1.4) * zoom,
                    highlighted: highlighted[index],
                }
            })
            .collect();
        Self { nodes, edges }
    }
}

fn graph_position(index: usize) -> (f32, f32) {
    if index == 0 {
        return (GRAPH_WIDTH / 2.0, GRAPH_HEIGHT / 2.0);
    }
    // Keep enough room between labels by filling progressively larger rings.
    let mut remaining = index - 1;
    let mut ring = 1usize;
    let mut ring_capacity = 8usize;
    while remaining >= ring_capacity {
        remaining -= ring_capacity;
        ring += 1;
        ring_capacity = 8 * ring;
    }
    let angle = (remaining as f32 / ring_capacity as f32) * std::f32::consts::TAU
        + if ring.is_multiple_of(2) {
            std::f32::consts::PI / ring_capacity as f32
        } else {
            0.0
        };
    let radius = 180.0 + (ring - 1) as f32 * 150.0;
    (
        GRAPH_WIDTH / 2.0 + angle.cos() * radius,
        GRAPH_HEIGHT / 2.0 + angle.sin() * radius * 0.72,
    )
}

#[cfg(test)]
#[path = "tests/graph_scene.rs"]
mod tests;
