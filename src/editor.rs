use super::*;

impl super::Emerald {
    pub(super) fn render_editor(
        &mut self,
        layout: ShellLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = EVERFOREST_DARK;
        let file_title = if self.interaction.renaming {
            self.interaction.rename_buffer.clone()
        } else {
            self.state
                .active_file()
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string()
        };
        let save_status = if self.state.is_dirty() {
            "Unsaved"
        } else {
            "Saved"
        };
        let parse_status = if self.file_loading() {
            "loading..."
        } else if let Some(error) = &self.interaction.file_load_error {
            error.as_str()
        } else if self.state.parse_is_dirty() {
            "parsing..."
        } else {
            "parsed"
        };
        let status = format!(
            "{} diagnostics | {parse_status} | {} lines | cursor {} | {}",
            self.state.parse_report().diagnostic_count,
            self.state.line_count(),
            self.state.cursor(),
            save_status
        );
        let text = self.state.text();
        let active_path = self.state.active_file().to_path_buf();
        let snapshot = self.render_cache.snapshot_for_revision(
            text,
            &active_path,
            self.state.text_revision(),
            (!self.state.parse_is_dirty()).then(|| self.state.parsed_document()),
        );
        let style = snapshot.map_or_else(RenderStyle::default, |snapshot| snapshot.style);
        let blocks = snapshot.map_or_else(Default::default, |snapshot| snapshot.blocks.clone());
        let prepared = snapshot.map_or_else(Default::default, |snapshot| snapshot.prepared.clone());
        let rows = snapshot.map_or_else(Default::default, |snapshot| snapshot.rows.clone());
        if self.document_list_count != rows.len()
            || self.document_list_file.as_ref() != Some(&active_path)
        {
            self.document_list_state.reset(rows.len());
            self.document_list_count = rows.len();
            self.document_list_file = Some(active_path.clone());
        }
        if self
            .document_list_blocks
            .as_ref()
            .is_some_and(|previous| !Arc::ptr_eq(previous, &blocks))
        {
            self.document_list_state.splice(0..rows.len(), rows.len());
        }
        self.document_list_blocks = Some(blocks.clone());
        let chrome = editor_chrome(layout);
        let scroll_chrome = document_scroll_chrome();
        let surface = document_surface_chrome();
        let document = div()
            .id("document-scroll")
            .track_focus(&self.focus_handle)
            .on_mouse_down(MouseButton::Left, cx.listener(Self::start_mouse_selection))
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(Self::show_editor_context_menu),
            )
            .tab_stop(true)
            .cursor_text()
            .flex()
            .flex_col()
            .gap_4()
            .flex_1()
            .w_full()
            .min_h(px(0.0))
            .min_w(px(0.0))
            .m(px(chrome.margin))
            .p(px(DOCUMENT_PADDING_X))
            .bg(rgb(style.panel.unwrap_or(theme.panel)))
            .border_1()
            .border_color(rgb(theme.panel_border))
            .rounded_lg()
            .text_color(rgb(style.text.unwrap_or(theme.text)))
            .text_lg();
        let document = if scroll_chrome.horizontal {
            document.overflow_x_scroll().overflow_y_scroll()
        } else {
            document.overflow_y_scroll()
        }
        .scrollbar_width(px(12.0));
        let document = if surface.fill_available_width && surface.fill_available_height {
            document.w_full().min_h(px(0.0))
        } else {
            document
        };

        div()
            .flex()
            .flex_col()
            .flex_1()
            .size_full()
            .bg(rgb(theme.background))
            .when_some(self.interaction.file_load_error.clone(), |panel, error| {
                panel.child(
                    div()
                        .mx(px(20.0))
                        .mt(px(8.0))
                        .p_2()
                        .bg(rgb(0x5b2727))
                        .text_color(rgb(0xffb4a8))
                        .child(error),
                )
            })
            .child(if self.interaction.graph_mode {
                self.render_graph(cx).into_any_element()
            } else {
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .w_full()
                    .min_h(px(0.0))
                    .min_w(px(0.0))
                    .child(
                        div()
                            .flex()
                            .justify_between()
                            .px_4()
                            .py_3()
                            .bg(rgb(theme.sidebar))
                            .border_b_1()
                            .border_color(rgb(theme.panel_border))
                            .text_color(rgb(theme.muted_text))
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .px_2()
                                    .border_1()
                                    .border_color(rgb(if self.interaction.renaming {
                                        theme.accent
                                    } else {
                                        theme.sidebar
                                    }))
                                    .rounded_sm()
                                    .cursor_text()
                                    .on_mouse_down(
                                        MouseButton::Left,
                                        cx.listener(|view, _, window, cx| {
                                            view.begin_rename(
                                                view.state.active_file().to_path_buf(),
                                                window,
                                                cx,
                                            );
                                        }),
                                    )
                                    .child(file_title)
                                    .when(self.interaction.renaming, |title| {
                                        title.child(div().w(px(2.0)).h(px(18.0)).bg(rgb(
                                            if self.blink_cursor.visible() {
                                                theme.accent
                                            } else {
                                                theme.sidebar
                                            },
                                        )))
                                    }),
                            )
                            .child(status),
                    )
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .w_full()
                            .min_h(px(0.0))
                            .min_w(px(0.0))
                            .child(
                                div()
                                    .flex()
                                    .flex_1()
                                    .w_full()
                                    .min_h(px(0.0))
                                    .min_w(px(0.0))
                                    .child(document.child(
                                        if self.interaction.source_revealed
                                            || self.state.selection().is_some()
                                        {
                                            self.render_source(cx.entity(), window)
                                        } else {
                                            let list_state = self.document_list_state.clone();
                                            let entity = cx.entity();
                                            let document_path = active_path.clone();
                                            list(list_state, move |index, _, _| {
                                                Self::render_preview_row(
                                                    index,
                                                    &rows[index],
                                                    &blocks,
                                                    style,
                                                    &prepared,
                                                    &document_path,
                                                    entity.clone(),
                                                )
                                            })
                                            .with_sizing_behavior(gpui::ListSizingBehavior::Auto)
                                            .w_full()
                                            .h_full()
                                            .into_any_element()
                                        },
                                    ))
                                    .child(
                                        div()
                                            .id("document-scrollbar-vertical")
                                            .w(px(12.0))
                                            .h_full()
                                            .bg(rgb(theme.sidebar))
                                            .border_l_1()
                                            .border_color(rgb(theme.panel_border))
                                            .cursor_pointer()
                                            .on_mouse_down(
                                                MouseButton::Left,
                                                cx.listener(Self::scroll_document_to_bottom),
                                            )
                                            .child(
                                                div().w_full().h(px(80.0)).bg(rgb(theme.accent)),
                                            ),
                                    ),
                            ),
                    )
                    .into_any_element()
            })
    }
}
