use super::graph_scene::{GraphScene, GRAPH_HEIGHT, GRAPH_WIDTH};
use super::*;
use gpui::{canvas, point, px, PathBuilder, SharedString};

fn should_open_graph_node(click_count: usize) -> bool {
    click_count == 2
}

fn note_name(path: &std::path::Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .replace('_', " ")
}

impl super::Emerald {
    pub(super) fn render_graph(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let graph = self.state.link_graph().clone();
        let active_path = self.state.active_file().to_path_buf();
        let hovered = self.interaction.graph_hovered;
        let zoom = self.interaction.graph_zoom;
        let offset = self.interaction.graph_offset;
        let scene = Arc::new(GraphScene::new(&graph, hovered, zoom));
        let edge_scene = scene.clone();

        let mut layer = div()
            .id("graph-layer")
            .absolute()
            .left(px(offset.x))
            .top(px(offset.y))
            .w(px(GRAPH_WIDTH * zoom))
            .h(px(GRAPH_HEIGHT * zoom))
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, _| {
                        for edge in &edge_scene.edges {
                            let source = &edge_scene.nodes[edge.source];
                            let target = &edge_scene.nodes[edge.target];
                            let start =
                                bounds.origin + point(px(source.center.0), px(source.center.1));
                            let end =
                                bounds.origin + point(px(target.center.0), px(target.center.1));
                            let mut line = PathBuilder::stroke(px(1.5 * zoom));
                            line.move_to(start);
                            if edge.source == edge.target {
                                // A self-reference needs a visible loop around its node.
                                let reach = px(source.radius * 4.0);
                                line.cubic_bezier_to(
                                    end,
                                    start + point(-reach, -reach),
                                    start + point(reach, -reach),
                                );
                            } else {
                                line.line_to(end);
                            }
                            match line.build() {
                                Ok(path) => window.paint_path(
                                    path,
                                    rgb(if edge.highlighted {
                                        theme.accent
                                    } else {
                                        theme.panel_border
                                    }),
                                ),
                                Err(error) => tracing::warn!(%error, "could not paint graph link"),
                            }
                        }
                    },
                )
                .absolute()
                .size_full(),
            );

        for (index, path) in graph.nodes.iter().enumerate() {
            let geometry = &scene.nodes[index];
            let (x, y) = geometry.center;
            let path = path.clone();
            let is_active = path == active_path;
            let is_highlighted = geometry.highlighted;
            let radius = geometry.radius;
            let mut node = div()
                .id(SharedString::from(format!("graph-node-{index}")))
                .absolute()
                .left(px(x - radius))
                .top(px(y - radius))
                .w(px(radius * 2.0))
                .h(px(radius * 2.0))
                .flex()
                .flex_col()
                .items_center()
                .cursor_pointer()
                .opacity(if is_highlighted { 1.0 } else { 0.4 })
                .text_xs()
                .text_color(rgb(if is_highlighted {
                    theme.text
                } else {
                    theme.faint_text
                }))
                .child(
                    div()
                        .w(px(radius * 2.0))
                        .h(px(radius * 2.0))
                        .flex_shrink_0()
                        .rounded_full()
                        .border_1()
                        .border_color(rgb(if is_active {
                            theme.accent
                        } else if path.exists() {
                            theme.muted_text
                        } else {
                            theme.destructive
                        }))
                        .bg(rgb(if is_active {
                            theme.accent
                        } else if path.exists() {
                            theme.panel
                        } else {
                            theme.destructive
                        })),
                )
                .child(
                    div()
                        .absolute()
                        .top(px(radius * 2.0 + 4.0))
                        .w(px(160.0))
                        .left(px(radius - 80.0))
                        .text_center()
                        .child(note_name(&path)),
                );
            node = node.on_hover(cx.listener(move |view, is_hovered: &bool, _, cx| {
                view.interaction.graph_hovered = if *is_hovered { Some(index) } else { None };
                cx.notify();
            }));
            layer = layer.child(node.on_click(cx.listener(
                move |view, event: &gpui::ClickEvent, window, cx| {
                    if should_open_graph_node(event.click_count()) {
                        view.open_graph_node(path.clone(), window, cx);
                    }
                },
            )));
        }

        let surface = div()
            .id("graph-canvas")
            .relative()
            .flex_1()
            .min_h(px(0.0))
            .overflow_hidden()
            .bg(rgb(theme.graph_background))
            .on_mouse_down(MouseButton::Middle, cx.listener(Self::graph_pan_start))
            .on_mouse_move(cx.listener(Self::graph_pan_move))
            .on_mouse_up(MouseButton::Middle, cx.listener(Self::graph_pan_end))
            .on_scroll_wheel(cx.listener(Self::graph_scroll))
            .child(layer);
        div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h(px(0.0))
            .bg(rgb(theme.graph_background))
            .child(surface)
            .child(
                div()
                    .absolute()
                    .top(px(14.0))
                    .left(px(18.0))
                    .flex()
                    .items_center()
                    .gap_2()
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(theme.menu))
                            .text_color(rgb(theme.text))
                            .child("Graph view"),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(theme.menu))
                            .text_color(rgb(theme.muted_text))
                            .child(format!(
                                "{} notes · {} links",
                                graph.nodes.len(),
                                graph.edges.len()
                            )),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top(px(14.0))
                    .right(px(18.0))
                    .flex()
                    .gap_2()
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(theme.menu))
                            .text_color(rgb(theme.muted_text))
                            .child("⌕  Search files"),
                    )
                    .child(
                        div()
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(theme.menu))
                            .text_color(rgb(theme.text))
                            .child(format!("{}%", (zoom * 100.0) as u32)),
                    )
                    .child(
                        div()
                            .id("graph-settings")
                            .role(gpui::Role::Button)
                            .aria_label("Graph settings")
                            .tab_stop(true)
                            .focus_visible(|button| button.bg(rgb(theme.hover)))
                            .px_3()
                            .py_2()
                            .rounded_md()
                            .bg(rgb(theme.menu))
                            .cursor_pointer()
                            .text_color(rgb(theme.text))
                            .child("⚙")
                            .on_click(cx.listener(Self::toggle_graph_settings)),
                    ),
            )
            .when(self.interaction.graph_settings, |panel| {
                panel.child(graph_settings(theme))
            })
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
#[path = "tests/graph_view.rs"]
mod tests;

fn graph_settings(theme: emerald::theme::EmeraldTheme) -> impl IntoElement {
    div()
        .absolute()
        .top(px(58.0))
        .right(px(18.0))
        .w(px(286.0))
        .p_4()
        .rounded_lg()
        .bg(rgb(theme.menu))
        .border_1()
        .border_color(rgb(theme.panel_border))
        .text_color(rgb(theme.text))
        .child(
            div()
                .flex()
                .justify_between()
                .child("Graph settings")
                .child(
                    div()
                        .text_xs()
                        .text_color(rgb(theme.muted_text))
                        .child("Restore defaults"),
                ),
        )
        .child(
            div()
                .mt_4()
                .text_xs()
                .text_color(rgb(theme.muted_text))
                .child("FILTERS"),
        )
        .child(
            div()
                .mt_2()
                .p_2()
                .bg(rgb(theme.sidebar))
                .text_color(rgb(theme.muted_text))
                .child("Search files"),
        )
        .child(
            div()
                .mt_3()
                .flex()
                .justify_between()
                .child("Tags")
                .child("○"),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child("Attachments")
                .child("○"),
        )
        .child(
            div()
                .flex()
                .justify_between()
                .child("Existing files only")
                .child("●"),
        )
        .child(div().flex().justify_between().child("Orphans").child("●"))
        .child(
            div()
                .mt_4()
                .text_xs()
                .text_color(rgb(theme.muted_text))
                .child("DISPLAY"),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .justify_between()
                .child("Arrows")
                .child("○"),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .justify_between()
                .child("Node size")
                .child("━━━━━━"),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .justify_between()
                .child("Link thickness")
                .child("━━"),
        )
        .child(
            div()
                .mt_4()
                .text_xs()
                .text_color(rgb(theme.muted_text))
                .child("FORCES"),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .justify_between()
                .child("Center force")
                .child("━━━━"),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .justify_between()
                .child("Repel force")
                .child("━━━━"),
        )
        .child(
            div()
                .mt_2()
                .flex()
                .justify_between()
                .child("Link distance")
                .child("━━━━"),
        )
}
