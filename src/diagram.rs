use std::collections::HashMap;

const CANVAS_WIDTH: f32 = 920.0;
const NODE_WIDTH: f32 = 250.0;
const NODE_HEIGHT: f32 = 64.0;
const NODE_GAP: f32 = 28.0;
const GROUP_GAP: f32 = 52.0;
const GROUP_PADDING: f32 = 18.0;
const GROUP_HEADER: f32 = 34.0;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct NodeStyle {
    fill: Option<String>,
    stroke: Option<String>,
    color: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Node {
    id: String,
    label: String,
    group: Option<usize>,
    class: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Group {
    title: String,
    nodes: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Edge {
    from: String,
    to: String,
    label: Option<String>,
    dotted: bool,
}

#[derive(Clone, Copy, Debug)]
struct NodeGeometry {
    x: f32,
    y: f32,
}

#[derive(Clone, Copy, Debug)]
struct GroupGeometry {
    x: f32,
    y: f32,
    width: f32,
    height: f32,
}

#[derive(Default)]
struct MermaidGraph {
    nodes: Vec<Node>,
    groups: Vec<Group>,
    edges: Vec<Edge>,
    styles: HashMap<String, NodeStyle>,
}

/// Render the supported Mermaid flowchart subset to a self-contained SVG.
///
/// Source mode retains the Mermaid text; preview mode uses this deterministic
/// renderer so it does not need a browser or JavaScript runtime.
pub fn mermaid_svg(source: &str) -> Option<String> {
    let graph = parse_mermaid(source);
    if graph.nodes.is_empty() {
        return None;
    }

    let (node_geometry, group_geometry, height) = layout(&graph);
    let mut svg = format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{CANVAS_WIDTH}" height="{height}" viewBox="0 0 {CANVAS_WIDTH} {height}">\
<defs><marker id="arrow" viewBox="0 0 10 10" refX="8" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M 0 0 L 10 5 L 0 10 z" fill="#94a3b8"/></marker></defs>\
<rect width="100%" height="100%" fill="#090b0d" rx="12"/>"##
    );

    for (group, geometry) in graph.groups.iter().zip(&group_geometry) {
        svg.push_str(&format!(
            r##"<rect x="{}" y="{}" width="{}" height="{}" rx="4" fill="#ffffff10" stroke="#64748b66"/>"##,
            geometry.x, geometry.y, geometry.width, geometry.height
        ));
        svg.push_str(&format!(
            r##"<text x="{}" y="{}" text-anchor="middle" fill="#cbd5e1" font-family="sans-serif" font-size="14">{}</text>"##,
            geometry.x + geometry.width / 2.0,
            geometry.y + 22.0,
            escape_xml(&group.title)
        ));
    }

    for (index, edge) in graph.edges.iter().enumerate() {
        let (Some(from), Some(to)) = (node_geometry.get(&edge.from), node_geometry.get(&edge.to))
        else {
            continue;
        };
        let (path, label_position) = edge_path(*from, *to, index);
        svg.push_str(&format!(
            r##"<path d="{path}" fill="none" stroke="#94a3b8" stroke-width="1.5" {} marker-end="url(#arrow)"/>"##,
            if edge.dotted { r#"stroke-dasharray="5 4""# } else { "" }
        ));
        if let Some(label) = &edge.label {
            let width = (label.chars().count() as f32 * 7.0 + 18.0).max(70.0);
            svg.push_str(&format!(
                r##"<rect x="{}" y="{}" width="{width}" height="24" rx="12" fill="#082f49" stroke="#0ea5e9"/><text x="{}" y="{}" text-anchor="middle" dominant-baseline="middle" fill="#7dd3fc" font-family="sans-serif" font-size="11">{}</text>"##,
                label_position.0 - width / 2.0,
                label_position.1 - 12.0,
                label_position.0,
                label_position.1 + 1.0,
                escape_xml(label)
            ));
        }
    }

    for node in &graph.nodes {
        let Some(geometry) = node_geometry.get(&node.id) else {
            continue;
        };
        let style = node
            .class
            .as_ref()
            .and_then(|class| graph.styles.get(class));
        let fill = style
            .and_then(|style| style.fill.as_deref())
            .unwrap_or("#111827");
        let stroke = style
            .and_then(|style| style.stroke.as_deref())
            .unwrap_or("#475569");
        let color = style
            .and_then(|style| style.color.as_deref())
            .unwrap_or("#e5e7eb");
        svg.push_str(&format!(
            r#"<rect x="{}" y="{}" width="{NODE_WIDTH}" height="{NODE_HEIGHT}" rx="12" fill="{}" stroke="{}" stroke-width="2"/>"#,
            geometry.x,
            geometry.y,
            escape_xml(fill),
            escape_xml(stroke)
        ));
        let lines = label_lines(&node.label);
        let first_y = geometry.y + NODE_HEIGHT / 2.0 - (lines.len() as f32 - 1.0) * 8.0;
        svg.push_str(&format!(
            r#"<text x="{}" y="{}" text-anchor="middle" fill="{}" font-family="sans-serif" font-size="12" font-weight="600">"#,
            geometry.x + NODE_WIDTH / 2.0,
            first_y,
            escape_xml(color)
        ));
        for (line_index, line) in lines.iter().enumerate() {
            svg.push_str(&format!(
                r#"<tspan x="{}" dy="{}">{}</tspan>"#,
                geometry.x + NODE_WIDTH / 2.0,
                if line_index == 0 { 0 } else { 16 },
                escape_xml(line)
            ));
        }
        svg.push_str("</text>");
    }

    svg.push_str("</svg>");
    Some(svg)
}

fn parse_mermaid(source: &str) -> MermaidGraph {
    let mut graph = MermaidGraph::default();
    let mut current_group = None;
    let mut classes = Vec::<(Vec<String>, String)>::new();

    for raw_line in source.lines() {
        let line = raw_line.trim().trim_end_matches(';');
        if line.is_empty() || line.starts_with("flowchart ") || line.starts_with("graph ") {
            continue;
        }
        if let Some(rest) = line.strip_prefix("subgraph ") {
            let (id, label) = parse_node(rest).unwrap_or_else(|| (rest.to_string(), rest.into()));
            let _ = id;
            graph.groups.push(Group {
                title: label,
                nodes: Vec::new(),
            });
            current_group = Some(graph.groups.len() - 1);
            continue;
        }
        if line == "end" {
            current_group = None;
            continue;
        }
        if let Some(rest) = line.strip_prefix("classDef ") {
            if let Some((name, values)) = rest.split_once(' ') {
                graph.styles.insert(name.into(), parse_style(values));
            }
            continue;
        }
        if let Some(rest) = line.strip_prefix("class ") {
            if let Some((ids, class)) = rest.rsplit_once(' ') {
                classes.push((
                    ids.split(',').map(|id| id.trim().into()).collect(),
                    class.into(),
                ));
            }
            continue;
        }
        if let Some(edge) = parse_edge(line) {
            ensure_node(&mut graph, &edge.from, None, current_group);
            ensure_node(&mut graph, &edge.to, None, current_group);
            graph.edges.push(edge);
            continue;
        }
        if let Some((id, label)) = parse_node(line) {
            ensure_node(&mut graph, &id, Some(label), current_group);
        }
    }

    for (ids, class) in classes {
        for id in ids {
            if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == id) {
                node.class = Some(class.clone());
            }
        }
    }
    graph
}

fn ensure_node(graph: &mut MermaidGraph, id: &str, label: Option<String>, group: Option<usize>) {
    if let Some(node) = graph.nodes.iter_mut().find(|node| node.id == id) {
        if let Some(label) = label {
            node.label = label;
        }
        if node.group.is_none() {
            node.group = group;
        }
    } else {
        graph.nodes.push(Node {
            id: id.into(),
            label: label.unwrap_or_else(|| id.into()),
            group,
            class: None,
        });
    }
    if let Some(group) = group {
        if !graph.groups[group].nodes.iter().any(|node| node == id) {
            graph.groups[group].nodes.push(id.into());
        }
    }
}

fn parse_node(value: &str) -> Option<(String, String)> {
    let value = value.trim();
    let id_end = value
        .find(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .unwrap_or(value.len());
    let id = value[..id_end].trim();
    if id.is_empty() {
        return None;
    }
    let label = value
        .find('"')
        .zip(value.rfind('"'))
        .filter(|(start, end)| start < end)
        .map(|(start, end)| value[start + 1..end].to_string())
        .or_else(|| {
            let start = value.find(['[', '('])?;
            let end = value.rfind([']', ')'])?;
            (start < end).then(|| {
                value[start + 1..end]
                    .trim_matches(['[', '(', ')', ']'])
                    .into()
            })
        })
        .unwrap_or_else(|| id.into());
    Some((id.into(), label))
}

fn parse_edge(line: &str) -> Option<Edge> {
    let (arrow, (left, right)) = ["-.->", "-->", "==>", "->"]
        .into_iter()
        .filter_map(|arrow| line.split_once(arrow).map(|parts| (arrow, parts)))
        .min_by_key(|(_, (left, _))| left.len())?;
    let (from, from_label) = parse_node(left)?;
    let right = right.trim();
    let (label, right) = if let Some(labelled) = right.strip_prefix('|') {
        let (label, endpoint) = labelled.split_once('|')?;
        (Some(label.trim().into()), endpoint.trim())
    } else {
        (None, right)
    };
    let (to, to_label) = parse_node(right)?;
    let _ = (from_label, to_label);
    Some(Edge {
        from,
        to,
        label,
        dotted: arrow.contains('.'),
    })
}

fn parse_style(values: &str) -> NodeStyle {
    let mut style = NodeStyle::default();
    for value in values.split(',') {
        let Some((name, value)) = value.split_once(':') else {
            continue;
        };
        match name.trim() {
            "fill" => style.fill = Some(value.trim().into()),
            "stroke" => style.stroke = Some(value.trim().into()),
            "color" => style.color = Some(value.trim().into()),
            _ => {}
        }
    }
    style
}

fn layout(graph: &MermaidGraph) -> (HashMap<String, NodeGeometry>, Vec<GroupGeometry>, f32) {
    let mut nodes = HashMap::new();
    let mut groups = Vec::new();
    let mut y = 24.0;

    for (group_index, group) in graph.groups.iter().enumerate() {
        let horizontal = group_index == 0 && group.nodes.len() > 1;
        let width = if horizontal {
            group.nodes.len() as f32 * NODE_WIDTH
                + (group.nodes.len().saturating_sub(1)) as f32 * NODE_GAP
                + GROUP_PADDING * 2.0
        } else {
            NODE_WIDTH + GROUP_PADDING * 2.0
        };
        let height = if horizontal {
            GROUP_HEADER + NODE_HEIGHT + GROUP_PADDING
        } else {
            GROUP_HEADER
                + group.nodes.len() as f32 * NODE_HEIGHT
                + group.nodes.len().saturating_sub(1) as f32 * NODE_GAP
                + GROUP_PADDING
        };
        let x = (CANVAS_WIDTH - width) / 2.0;
        groups.push(GroupGeometry {
            x,
            y,
            width,
            height,
        });
        for (node_index, id) in group.nodes.iter().enumerate() {
            let node_x = if horizontal {
                x + GROUP_PADDING + node_index as f32 * (NODE_WIDTH + NODE_GAP)
            } else {
                x + GROUP_PADDING
            };
            let node_y = if horizontal {
                y + GROUP_HEADER
            } else {
                y + GROUP_HEADER + node_index as f32 * (NODE_HEIGHT + NODE_GAP)
            };
            nodes.insert(
                id.clone(),
                NodeGeometry {
                    x: node_x,
                    y: node_y,
                },
            );
        }
        y += height + GROUP_GAP;
    }

    for node in graph.nodes.iter().filter(|node| node.group.is_none()) {
        nodes.insert(
            node.id.clone(),
            NodeGeometry {
                x: (CANVAS_WIDTH - NODE_WIDTH) / 2.0,
                y,
            },
        );
        y += NODE_HEIGHT + NODE_GAP;
    }

    (nodes, groups, (y - GROUP_GAP + 24.0).max(160.0))
}

fn edge_path(from: NodeGeometry, to: NodeGeometry, edge_index: usize) -> (String, (f32, f32)) {
    let from_center = from.x + NODE_WIDTH / 2.0;
    let to_center = to.x + NODE_WIDTH / 2.0;
    let start_y = from.y + NODE_HEIGHT;
    let end_y = to.y;
    if end_y > start_y {
        let middle_y = start_y + (end_y - start_y) / 2.0;
        (
            format!(
                "M {from_center} {start_y} L {from_center} {middle_y} L {to_center} {middle_y} L {to_center} {end_y}"
            ),
            ((from_center + to_center) / 2.0, middle_y),
        )
    } else {
        let gutter = CANVAS_WIDTH - 24.0 - (edge_index % 4) as f32 * 10.0;
        let from_x = from.x + NODE_WIDTH;
        let to_x = to.x + NODE_WIDTH;
        let from_y = from.y + NODE_HEIGHT / 2.0;
        let to_y = to.y + NODE_HEIGHT / 2.0;
        (
            format!("M {from_x} {from_y} L {gutter} {from_y} L {gutter} {to_y} L {to_x} {to_y}"),
            (gutter, (from_y + to_y) / 2.0),
        )
    }
}

fn label_lines(label: &str) -> Vec<&str> {
    label
        .split("<br/>")
        .filter(|line| !line.is_empty())
        .collect()
}

fn escape_xml(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_grouped_mermaid_flowchart_as_svg() {
        let source = r##"flowchart TD
subgraph Sources["Sources"]
A["Input<br/>One"]
B["Input Two"]
end
subgraph Output["Output"]
C["Result"]
end
A --> C
B -.->|Updated| C
classDef source fill:#e0f2fe,stroke:#0284c7,color:#0c4a6e
class A,B source"##;

        let svg = mermaid_svg(source).unwrap();
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("Sources"));
        assert!(svg.contains("Input"));
        assert!(svg.contains("stroke-dasharray"));
        assert!(svg.contains("Updated"));
        assert!(svg.contains("#e0f2fe"));
        assert!(!svg.contains("flowchart TD"));
    }
}
