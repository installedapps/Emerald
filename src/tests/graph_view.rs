use super::should_open_graph_node;

#[test]
fn graph_nodes_open_only_on_double_click() {
    assert!(!should_open_graph_node(1));
    assert!(should_open_graph_node(2));
    assert!(!should_open_graph_node(3));
}
