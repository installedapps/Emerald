use super::*;

impl super::Emerald {
    pub(super) fn render_sidebar(
        &self,
        layout: ShellLayout,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme();
        let width = self.interaction.sidebar_width;
        let workspace = self.state.workspace_root().display().to_string();

        let sidebar = div()
            .relative()
            .flex()
            .flex_col()
            .gap_3()
            .w(px(width))
            .p_3()
            .bg(rgb(theme.sidebar))
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
                            view.render_sidebar_file(view.state.files()[index].clone(), index, cx)
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
                    .gap_2()
                    .child(
                        toolbar_button("new-file", "＋", "Create note", theme, true)
                            .on_click(cx.listener(Self::create_new_file)),
                    )
                    .child(
                        toolbar_button("save", "⇩", "Save note", theme, false)
                            .on_click(cx.listener(Self::save_current_file)),
                    )
                    .child(
                        toolbar_button("open", "↗", "Open note", theme, false)
                            .on_click(cx.listener(Self::open_file)),
                    )
                    .child(
                        toolbar_button("delete", "⌫", "Delete note", theme, false)
                            .on_click(cx.listener(Self::delete_current_file)),
                    )
                    .child(
                        toolbar_button(
                            "graph",
                            if self.interaction.graph_mode {
                                "◉"
                            } else {
                                "◎"
                            },
                            "Toggle graph view",
                            theme,
                            self.interaction.graph_mode,
                        )
                        .on_click(cx.listener(Self::toggle_graph)),
                    ),
            )
            .child(
                div()
                    .min_w(px(0.0))
                    .overflow_hidden()
                    .truncate()
                    .text_xs()
                    .text_color(rgb(theme.muted_text))
                    .child(workspace),
            )
            .child(file_list);
        self.sidebar_frame(false, sidebar, cx)
    }

    pub(super) fn render_sidebar_file(
        &self,
        file: PathBuf,
        index: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme();
        let is_active = file == self.state.active_file();
        let style = sidebar_file_style(theme, is_active, false);
        let hover_style = sidebar_file_style(theme, is_active, true);
        let file_path = file.clone();
        let context_file = file.clone();

        div()
            .id(("sidebar-file", index))
            .role(gpui::Role::Button)
            .aria_label(
                file.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            )
            .tab_stop(true)
            .focus_visible(|row| row.border_color(rgb(theme.accent)))
            .h(px(30.0))
            .w_full()
            .overflow_hidden()
            .p_2()
            .border_1()
            .border_color(rgb(style.border))
            .bg(rgb(style.background))
            .text_color(rgb(style.text))
            .cursor_pointer()
            .rounded_md()
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
            .hover(|file| {
                file.bg(rgb(hover_style.background))
                    .border_color(rgb(hover_style.border))
                    .text_color(rgb(hover_style.text))
            })
            .child(
                file.file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string(),
            )
    }
}

fn toolbar_button(
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    theme: emerald::theme::EmeraldTheme,
    active: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
        .role(gpui::Role::Button)
        .aria_label(label)
        .tab_stop(true)
        .focus_visible(|button| button.border_color(rgb(theme.accent)))
        .w(px(32.0))
        .h(px(32.0))
        .flex()
        .items_center()
        .justify_center()
        .rounded_md()
        .border_1()
        .border_color(rgb(if active {
            theme.accent
        } else {
            theme.panel_border
        }))
        .bg(rgb(if active {
            theme.active_file
        } else {
            theme.panel
        }))
        .text_color(rgb(if active {
            theme.active_file_text
        } else {
            theme.muted_text
        }))
        .text_lg()
        .cursor_pointer()
        .hover(|button| {
            button
                .border_color(rgb(theme.accent))
                .bg(rgb(theme.active_file))
                .text_color(rgb(theme.text))
        })
        .child(icon)
}
