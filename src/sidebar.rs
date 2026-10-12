use super::*;
use gpui::{canvas, point, PathBuilder};

#[derive(Clone, Copy)]
pub(super) enum SidebarIcon {
    NewNote,
    Save,
    Open,
    Delete,
    Graph,
    Tag,
    NotesPanel { links: bool, visible: bool },
}

impl super::Emerald {
    pub(super) fn render_sidebar(
        &mut self,
        layout: ShellLayout,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme();
        let width = self.interaction.sidebar_width;
        let workspace = self.state.workspace_root().display().to_string();
        let tags_by_file = self
            .state
            .workspace_index()
            .notes()
            .map(|note| (note.path.clone(), super::tags::extract_tags(&note.text)))
            .collect::<std::collections::HashMap<_, _>>();

        let sidebar = div()
            .relative()
            .flex()
            .flex_col()
            .gap_3()
            .w(px(width))
            .p_3()
            .bg(sidebar_background(theme))
            .border_r_1()
            .border_color(rgb(theme.panel_border));
        let sidebar = if let Some(height) = layout.sidebar_height {
            sidebar.h(px(height))
        } else {
            sidebar.h_full()
        };

        let entity = cx.entity();
        let file_list = uniform_list(
            "sidebar-file-list",
            self.state.files().len(),
            move |range, _, cx| {
                entity.update(cx, |view, cx| {
                    range
                        .map(|index| {
                            let file = view.state.files()[index].clone();
                            let tags = tags_by_file.get(&file).cloned().unwrap_or_default();
                            view.render_sidebar_file(file, tags, index, cx)
                                .into_any_element()
                        })
                        .collect()
                })
            },
        )
        .flex_1()
        .min_h(px(0.0))
        .w_full();

        let sidebar = sidebar
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child(
                        div()
                            .text_sm()
                            .text_color(rgb(theme.accent))
                            .child("EMERALD"),
                    )
                    .child(
                        div()
                            .id("theme-picker")
                            .role(gpui::Role::Button)
                            .aria_label("Choose color theme")
                            .tab_stop(true)
                            .focus_visible(|button| {
                                button.bg(rgb(theme.hover)).text_color(rgb(theme.text))
                            })
                            .px_2()
                            .py_1()
                            .rounded_md()
                            .text_xs()
                            .text_color(rgb(theme.muted_text))
                            .hover(|button| button.bg(rgb(theme.hover)).text_color(rgb(theme.text)))
                            .cursor_pointer()
                            .child(format!("◐ {}", theme.name))
                            .on_click(cx.listener(|view, _, _, cx| {
                                cx.stop_propagation();
                                view.interaction.context_menu = Some(ContextMenu::Themes {
                                    position: gpui::point(px(220.0), px(80.0)),
                                });
                                cx.notify();
                            })),
                    ),
            )
            .child(
                div()
                    .flex()
                    .flex_1()
                    .min_h(px(0.0))
                    .gap_3()
                    .child(
                        div()
                            .id("sidebar-action-rail")
                            .debug_selector(|| "sidebar-action-rail".into())
                            .flex()
                            .flex_col()
                            .gap_2()
                            .child(
                                toolbar_button(
                                    "new-file",
                                    SidebarIcon::NewNote,
                                    "Create note",
                                    theme,
                                    false,
                                )
                                .on_click(cx.listener(Self::create_new_file)),
                            )
                            .child(
                                toolbar_button(
                                    "save",
                                    SidebarIcon::Save,
                                    "Save note",
                                    theme,
                                    false,
                                )
                                .on_click(cx.listener(Self::save_current_file)),
                            )
                            .child(
                                toolbar_button(
                                    "open",
                                    SidebarIcon::Open,
                                    "Open note",
                                    theme,
                                    false,
                                )
                                .on_click(cx.listener(Self::open_file)),
                            )
                            .child(
                                toolbar_button(
                                    "delete",
                                    SidebarIcon::Delete,
                                    "Delete note",
                                    theme,
                                    false,
                                )
                                .on_click(cx.listener(Self::delete_current_file)),
                            )
                            .child(
                                toolbar_button(
                                    "graph",
                                    SidebarIcon::Graph,
                                    "Toggle graph view",
                                    theme,
                                    self.interaction.graph_mode,
                                )
                                .on_click(cx.listener(Self::toggle_graph)),
                            ),
                    )
                    .child(
                        div()
                            .id("sidebar-file-column")
                            .debug_selector(|| "sidebar-file-column".into())
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w(px(0.0))
                            .min_h(px(0.0))
                            .gap_3()
                            .child(
                                div()
                                    .min_w(px(0.0))
                                    .overflow_hidden()
                                    .truncate()
                                    .text_xs()
                                    .text_color(rgb(theme.muted_text))
                                    .child(workspace),
                            )
                            .child(file_list),
                    ),
            );
        self.sidebar_frame(false, sidebar, cx)
    }

    pub(super) fn render_sidebar_file(
        &self,
        file: PathBuf,
        tags: Vec<String>,
        index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme();
        let is_active = file == self.state.active_file();
        let style = sidebar_file_style(theme, is_active, false);
        let hover_style = sidebar_file_style(theme, is_active, true);
        let file_path = file.clone();
        let context_file = file.clone();
        let file_name = file
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let accessible_name = if tags.is_empty() {
            file_name.clone()
        } else {
            format!("{file_name}, tags: {}", tags.join(", "))
        };
        let mut visible_tags = tags.iter().take(3).cloned().collect::<Vec<_>>();
        if tags.len() > 3 {
            visible_tags.push(format!("+{}", tags.len() - 3));
        }

        div()
            .id(("sidebar-file", index))
            .role(gpui::Role::Button)
            .aria_label(accessible_name)
            .tab_stop(true)
            .focus_visible(|row| row.border_color(rgb(theme.accent)))
            .h(px(44.0))
            .w_full()
            .pb(px(4.0))
            .border_1()
            .border_color(rgb(theme.sidebar))
            .cursor_pointer()
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, event, window, cx| {
                    this.select_sidebar_file(file_path.clone(), event, window, cx);
                }),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event, window, cx| {
                    this.show_sidebar_context_menu(context_file.clone(), event, window, cx);
                }),
            )
            .child(
                div()
                    .h_full()
                    .w_full()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .flex()
                    .flex_col()
                    .justify_center()
                    .gap_1()
                    .px_2()
                    .border_1()
                    .border_color(rgb(style.border))
                    .bg(rgb(style.background))
                    .text_color(rgb(style.text))
                    .rounded_md()
                    .hover(|row| {
                        row.bg(rgb(hover_style.background))
                            .border_color(rgb(hover_style.border))
                            .text_color(rgb(hover_style.text))
                    })
                    .child(div().w_full().truncate().text_xs().child(file_name))
                    .child(
                        div()
                            .h(px(15.0))
                            .flex()
                            .items_center()
                            .gap_1()
                            .overflow_hidden()
                            .children(visible_tags.into_iter().map(|tag| {
                                div()
                                    .max_w(px(60.0))
                                    .overflow_hidden()
                                    .truncate()
                                    .px_1()
                                    .py(px(1.0))
                                    .text_xs()
                                    .rounded_full()
                                    .bg(rgb(theme.accent).opacity(0.14))
                                    .text_color(rgb(theme.accent))
                                    .child(if tag.starts_with('+') {
                                        tag
                                    } else {
                                        format!("#{tag}")
                                    })
                            })),
                    ),
            )
    }
}

fn toolbar_button(
    id: &'static str,
    icon: SidebarIcon,
    label: &'static str,
    theme: emerald::theme::EmeraldTheme,
    active: bool,
) -> gpui::Stateful<gpui::Div> {
    let glow: gpui::Hsla = rgb(theme.accent).into();
    div()
        .id(id)
        .debug_selector(|| id.into())
        .role(gpui::Role::Button)
        .aria_label(label)
        .tab_stop(true)
        .focus_visible(|button| {
            button
                .bg(rgb(theme.hover))
                .border_color(rgb(theme.accent))
                .text_color(rgb(theme.text))
        })
        .relative()
        .w(px(28.0))
        .h(px(28.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(rgb(theme.sidebar))
        .bg(rgb(if active { theme.hover } else { theme.sidebar }))
        .text_color(rgb(if active {
            theme.accent
        } else {
            theme.muted_text
        }))
        .text_base()
        .cursor_pointer()
        .hover(|button| {
            button
                .border_color(rgb(theme.panel_border))
                .bg(rgb(theme.hover))
                .text_color(rgb(theme.text))
                .shadow(vec![gpui::BoxShadow::new(
                    px(0.0),
                    px(0.0),
                    glow.opacity(0.18),
                )
                .blur_radius(px(7.0))])
        })
        .child(sidebar_icon(icon, theme, active, 18.0))
}

pub(super) fn sidebar_icon(
    icon: SidebarIcon,
    theme: emerald::theme::EmeraldTheme,
    active: bool,
    size: f32,
) -> impl IntoElement {
    let stroke = if active {
        theme.accent
    } else {
        theme.muted_text
    };
    let crack = theme.accent;
    canvas(
        |_, _, _| (),
        move |bounds, _, window, _| {
            let origin = bounds.origin;
            let p = |x: f32, y: f32| origin + point(px(x), px(y));
            let mut paths = Vec::new();
            let mut cracks = Vec::new();

            match icon {
                SidebarIcon::NewNote => {
                    paths.push(vec![(8.0, 2.0), (8.0, 14.0)]);
                    paths.push(vec![(2.0, 8.0), (14.0, 8.0)]);
                    cracks.push(vec![(7.0, 6.0), (9.0, 8.0), (7.0, 10.0)]);
                }
                SidebarIcon::Save => {
                    paths.push(vec![
                        (3.0, 2.0),
                        (11.0, 2.0),
                        (14.0, 5.0),
                        (14.0, 14.0),
                        (2.0, 14.0),
                        (2.0, 2.0),
                        (3.0, 2.0),
                    ]);
                    paths.push(vec![(5.0, 2.0), (5.0, 6.0), (10.0, 6.0), (10.0, 2.0)]);
                    paths.push(vec![
                        (5.0, 9.0),
                        (11.0, 9.0),
                        (11.0, 14.0),
                        (5.0, 14.0),
                        (5.0, 9.0),
                    ]);
                    cracks.push(vec![(8.0, 6.0), (7.0, 8.0), (9.0, 10.0)]);
                }
                SidebarIcon::Open => {
                    paths.push(vec![
                        (2.0, 5.0),
                        (2.0, 4.0),
                        (6.0, 4.0),
                        (8.0, 6.0),
                        (14.0, 6.0),
                        (13.0, 13.0),
                        (2.0, 13.0),
                        (2.0, 5.0),
                    ]);
                    paths.push(vec![(3.0, 8.0), (13.0, 8.0)]);
                    cracks.push(vec![(7.0, 6.0), (6.0, 8.0), (8.0, 10.0)]);
                }
                SidebarIcon::Delete => {
                    paths.push(vec![(3.0, 4.0), (13.0, 4.0)]);
                    paths.push(vec![(6.0, 2.0), (10.0, 2.0)]);
                    paths.push(vec![(4.0, 5.0), (5.0, 14.0), (11.0, 14.0), (12.0, 5.0)]);
                    paths.push(vec![(7.0, 7.0), (7.0, 12.0)]);
                    paths.push(vec![(9.0, 7.0), (9.0, 12.0)]);
                    cracks.push(vec![(5.0, 8.0), (8.0, 9.0), (7.0, 11.0)]);
                }
                SidebarIcon::Graph => {
                    paths.push(vec![(5.0, 5.0), (11.0, 5.0), (8.0, 11.0), (5.0, 5.0)]);
                    for (x, y) in [(4.0, 4.0), (12.0, 4.0), (8.0, 12.0)] {
                        let mut circle = PathBuilder::stroke(px(1.3));
                        circle.move_to(p(x, y - 2.0));
                        circle.cubic_bezier_to(
                            p(x + 2.0, y),
                            p(x + 1.1, y - 2.0),
                            p(x + 2.0, y - 1.1),
                        );
                        circle.cubic_bezier_to(
                            p(x, y + 2.0),
                            p(x + 2.0, y + 1.1),
                            p(x + 1.1, y + 2.0),
                        );
                        circle.cubic_bezier_to(
                            p(x - 2.0, y),
                            p(x - 1.1, y + 2.0),
                            p(x - 2.0, y + 1.1),
                        );
                        circle.cubic_bezier_to(
                            p(x, y - 2.0),
                            p(x - 2.0, y - 1.1),
                            p(x - 1.1, y - 2.0),
                        );
                        if let Ok(path) = circle.build() {
                            window.paint_path(path, rgb(stroke));
                        }
                    }
                    cracks.push(vec![(7.0, 5.0), (8.0, 7.0), (7.0, 9.0)]);
                }
                SidebarIcon::Tag => {
                    paths.push(vec![
                        (2.5, 7.0),
                        (7.0, 2.5),
                        (14.0, 2.5),
                        (14.0, 9.5),
                        (9.0, 14.0),
                        (2.5, 7.5),
                        (2.5, 7.0),
                    ]);
                    paths.push(vec![
                        (5.0, 6.0),
                        (5.4, 5.2),
                        (6.2, 5.0),
                        (7.0, 5.5),
                        (7.0, 6.3),
                        (6.4, 7.0),
                        (5.5, 6.8),
                        (5.0, 6.0),
                    ]);
                }
                SidebarIcon::NotesPanel { links, visible } => {
                    paths.push(vec![
                        (2.0, 2.0),
                        (14.0, 2.0),
                        (14.0, 14.0),
                        (2.0, 14.0),
                        (2.0, 2.0),
                    ]);
                    let divider = match (links, visible) {
                        (false, true) => 6.0,
                        (false, false) => 4.0,
                        (true, true) => 10.0,
                        (true, false) => 12.0,
                    };
                    paths.push(vec![(divider, 2.0), (divider, 14.0)]);
                    cracks.push(vec![
                        (divider, 7.0),
                        (divider - 1.0, 8.0),
                        (divider + 1.0, 10.0),
                    ]);
                }
            }

            for points in paths {
                paint_icon_path(&points, &p, stroke, 1.35, window, 1.0);
            }
            for points in cracks {
                paint_icon_path(&points, &p, crack, 3.0, window, 0.14);
                paint_icon_path(
                    &points,
                    &p,
                    crack,
                    1.0,
                    window,
                    if active { 0.95 } else { 0.52 },
                );
            }
        },
    )
    .w(px(size))
    .h(px(size))
}

fn paint_icon_path(
    points: &[(f32, f32)],
    point: &impl Fn(f32, f32) -> gpui::Point<gpui::Pixels>,
    color: u32,
    width: f32,
    window: &mut Window,
    opacity: f32,
) {
    let Some(&(x, y)) = points.first() else {
        return;
    };
    let mut path = PathBuilder::stroke(px(width));
    path.move_to(point(x, y));
    for &(x, y) in &points[1..] {
        path.line_to(point(x, y));
    }
    if let Ok(path) = path.build() {
        let color: gpui::Hsla = rgb(color).into();
        window.paint_path(path, color.opacity(opacity));
    }
}
