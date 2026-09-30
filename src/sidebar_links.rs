use super::*;
use gpui_component::scroll::ScrollableElement;

impl super::Emerald {
    pub(super) fn render_links_sidebar(&mut self, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let active = self.state.active_file().to_path_buf();
        let targets = outgoing_targets(self.state.link_graph(), &active);
        let items = targets
            .into_iter()
            .map(|path| {
                let target = note_target(&path).expect("graph links are AsciiDoc notes");
                let label = note_label(&path);
                div()
                    .id(gpui::SharedString::from(format!("linked-note-{target}")))
                    .role(gpui::Role::Button)
                    .aria_label(format!("Open linked note {label}"))
                    .tab_stop(true)
                    .w_full()
                    .px_2()
                    .py_2()
                    .rounded_md()
                    .bg(rgb(theme.panel))
                    .text_color(rgb(theme.text))
                    .cursor_pointer()
                    .hover(|row| row.bg(rgb(theme.hover)).text_color(rgb(theme.text)))
                    .child(label)
                    .on_click(cx.listener(move |view, _, window, cx| {
                        view.open_link(target.clone(), window, cx)
                    }))
                    .into_any_element()
            })
            .collect::<Vec<_>>();
        let width = self.interaction.links_sidebar_width;
        let sidebar = div()
            .flex()
            .flex_col()
            .gap_2()
            .w(px(width))
            .h_full()
            .min_h(px(0.0))
            .overflow_y_scrollbar()
            .p_3()
            .bg(rgb(theme.sidebar))
            .border_l_1()
            .border_color(rgb(theme.panel_border))
            .child(div().text_sm().text_color(rgb(theme.accent)).child("LINKS"))
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme.muted_text))
                    .child("Outgoing links"),
            );
        let sidebar = if items.is_empty() {
            sidebar.child(
                div()
                    .py_2()
                    .text_sm()
                    .text_color(rgb(theme.muted_text))
                    .child("No linked notes"),
            )
        } else {
            items
                .into_iter()
                .fold(sidebar, |panel, item| panel.child(item))
        };
        self.sidebar_frame(true, sidebar, cx)
    }
}

pub(super) fn outgoing_targets(
    graph: &emerald::graph::LinkGraph,
    active: &std::path::Path,
) -> Vec<std::path::PathBuf> {
    let mut targets = Vec::new();
    for edge in &graph.edges {
        if edge.source == active && !targets.contains(&edge.target) {
            targets.push(edge.target.clone());
        }
    }
    targets
}

pub(super) fn note_target(path: &std::path::Path) -> Option<String> {
    path.file_name()?
        .to_str()
        .filter(|name| name.ends_with(".adoc"))
        .map(str::to_owned)
}

pub(super) fn note_label(path: &std::path::Path) -> String {
    path.file_stem()
        .unwrap_or_default()
        .to_string_lossy()
        .replace('_', " ")
}
