use super::*;

impl super::Emerald {
    pub(super) fn render_sidebar(
        &self,
        layout: ShellLayout,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = EVERFOREST_DARK;
        let workspace = self.state.workspace_root().display().to_string();

        let sidebar = div()
            .flex()
            .flex_col()
            .gap_3()
            .w(px(layout.sidebar_width))
            .p_4()
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
                            view.render_sidebar_file(view.state.files()[index].clone(), cx)
                                .into_any_element()
                        })
                        .collect()
                })
            },
        )
        .flex_1()
        .min_h(px(0.0))
        .w_full();

        sidebar
            .child(
                div()
                    .text_sm()
                    .text_color(rgb(theme.accent))
                    .child("EMERALD"),
            )
            .child(
                div()
                    .flex()
                    .gap_1()
                    .child(
                        toolbar_button("new-file", "＋", theme, true)
                            .on_mouse_down(MouseButton::Left, cx.listener(Self::create_new_file)),
                    )
                    .child(
                        toolbar_button("save", "⇩", theme, false)
                            .on_mouse_down(MouseButton::Left, cx.listener(Self::save_current_file)),
                    )
                    .child(
                        toolbar_button("open", "↗", theme, false)
                            .on_mouse_down(MouseButton::Left, cx.listener(Self::open_file)),
                    )
                    .child(
                        toolbar_button("delete", "⌫", theme, false).on_mouse_down(
                            MouseButton::Left,
                            cx.listener(Self::delete_current_file),
                        ),
                    )
                    .child(
                        toolbar_button(
                            "graph",
                            if self.interaction.graph_mode {
                                "◉"
                            } else {
                                "◎"
                            },
                            theme,
                            self.interaction.graph_mode,
                        )
                        .on_mouse_down(MouseButton::Left, cx.listener(Self::toggle_graph)),
                    ),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme.muted_text))
                    .child(workspace),
            )
            .child(file_list)
    }

    pub(super) fn render_sidebar_file(
        &self,
        file: PathBuf,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = EVERFOREST_DARK;
        let is_active = file == self.state.active_file();
        let style = sidebar_file_style(theme, is_active);
        let file_path = file.clone();
        let context_file = file.clone();

        div()
            .h(px(40.0))
            .w_full()
            .overflow_hidden()
            .p_2()
            .border_1()
            .border_color(rgb(style.border))
            .bg(rgb(style.background))
            .text_color(rgb(style.text))
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
    theme: emerald::theme::EmeraldTheme,
    active: bool,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(id)
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
            theme.accent
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
