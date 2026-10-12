use super::*;
use gpui::AnimationExt;

impl super::Emerald {
    pub(super) fn render_editor(
        &mut self,
        layout: ShellLayout,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme();
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
        let active_path = self.state.active_file().to_path_buf();
        let note_tags = self
            .state
            .workspace_index()
            .note(&active_path)
            .map(|note| super::tags::extract_tags(&note.text))
            .unwrap_or_default();
        let text = self.state.text();
        let source_view_active = match self.interaction.editor_view_mode {
            EditorViewMode::Source => true,
            EditorViewMode::Preview => false,
            EditorViewMode::Automatic => {
                self.interaction.source_revealed || self.state.selection().is_some()
            }
        };
        let scrollbar_state = if source_view_active {
            self.source_list_state.clone()
        } else {
            self.document_list_state.clone()
        };
        let accent_color = rgb(theme.accent);
        let accent_glow: gpui::Hsla = accent_color.into();
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
        let visible_rows = if self.interaction.headings_folded {
            let mut in_section = false;
            rows.iter()
                .filter(|row| match blocks.get(row.block_index) {
                    Some(RenderBlock::Heading { .. }) => {
                        in_section = true;
                        true
                    }
                    _ => !in_section,
                })
                .cloned()
                .collect::<Vec<_>>()
        } else {
            rows.as_ref().clone()
        };
        let first_heading_row = visible_rows.iter().position(|row| {
            matches!(
                blocks.get(row.block_index),
                Some(RenderBlock::Heading { .. })
            )
        });
        if self.document_list_count != visible_rows.len()
            || self.document_list_file.as_ref() != Some(&active_path)
        {
            self.document_list_state.reset(visible_rows.len());
            self.document_list_count = visible_rows.len();
            self.document_list_file = Some(active_path.clone());
        }
        if self
            .document_list_blocks
            .as_ref()
            .is_some_and(|previous| !Arc::ptr_eq(previous, &blocks))
        {
            self.document_list_state
                .splice(0..visible_rows.len(), visible_rows.len());
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
            .max_w(px(980.0))
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
        let revision = self.interaction.note_transition_revision;
        let document = document.with_animation(
            gpui::SharedString::from(format!("note-content-{revision}")),
            gpui::Animation::new(Duration::from_millis(180)),
            move |element, phase| {
                if revision == 0 {
                    element
                } else {
                    element
                        .opacity(phase)
                        .mt(px(chrome.margin + 6.0 * (1.0 - phase)))
                }
            },
        );

        div()
            .flex()
            .flex_col()
            .flex_1()
            .min_w(px(0.0))
            .when(layout.mode == LayoutMode::Desktop, |panel| panel.h_full())
            .bg(rgb(theme.window_background))
            .when_some(self.interaction.file_load_error.clone(), |panel, error| {
                panel.child(
                    div()
                        .mx(px(20.0))
                        .mt(px(8.0))
                        .p_2()
                        .bg(rgb(theme.destructive))
                        .text_color(rgb(theme.text))
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
                                    .gap_2()
                                    .child(self.sidebar_toggle_button(false, cx))
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
                                    ),
                            )
                            .child(
                                div()
                                    .flex()
                                    .items_center()
                                    .gap_2()
                                    .child(status)
                                    .child(
                                        div()
                                            .id("source-preview-toggle")
                                            .role(gpui::Role::Button)
                                            .aria_label(if source_view_active {
                                                "Show live preview"
                                            } else {
                                                "Edit source"
                                            })
                                            .tab_stop(true)
                                            .focus_visible(|button| {
                                                button
                                                    .bg(rgb(theme.hover))
                                                    .text_color(rgb(theme.text))
                                            })
                                            .px_2()
                                            .py_1()
                                            .rounded_md()
                                            .text_sm()
                                            .text_color(rgb(if source_view_active {
                                                theme.accent
                                            } else {
                                                theme.muted_text
                                            }))
                                            .hover(|button| {
                                                button
                                                    .bg(rgb(theme.hover))
                                                    .text_color(rgb(theme.text))
                                            })
                                            .cursor_pointer()
                                            .child("✎")
                                            .on_click(cx.listener(|view, _, _, cx| {
                                                view.toggle_editor_view(cx);
                                            })),
                                    )
                                    .child(self.sidebar_toggle_button(true, cx)),
                            ),
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
                                    .relative()
                                    .flex()
                                    .flex_1()
                                    .justify_center()
                                    .w_full()
                                    .min_h(px(0.0))
                                    .min_w(px(0.0))
                                    .child(document.child(if source_view_active {
                                        self.render_source(cx.entity(), window)
                                    } else {
                                        let list_state = self.document_list_state.clone();
                                        let entity = cx.entity();
                                        let document_path = active_path.clone();
                                        let note_tags = note_tags.clone();
                                        list(list_state, move |index, _, _| {
                                            let row = Self::render_preview_row(
                                                index,
                                                &visible_rows[index],
                                                &blocks,
                                                style,
                                                theme,
                                                &prepared,
                                                &document_path,
                                                entity.clone(),
                                            );
                                            if Some(index) == first_heading_row
                                                && !note_tags.is_empty()
                                            {
                                                div()
                                                    .w_full()
                                                    .flex()
                                                    .flex_col()
                                                    .gap_2()
                                                    .child(row)
                                                    .child(super::tags::render_note_tags(
                                                        &note_tags, theme,
                                                    ))
                                                    .into_any_element()
                                            } else {
                                                row
                                            }
                                        })
                                        .with_sizing_behavior(gpui::ListSizingBehavior::Auto)
                                        .w_full()
                                        .h_full()
                                        .into_any_element()
                                    }))
                                    .child(
                                        div()
                                            .absolute()
                                            .top_0()
                                            .bottom_0()
                                            .right_0()
                                            .w(gpui_base::Scrollbar::width())
                                            .hover(|rail| {
                                                rail.shadow(vec![gpui::BoxShadow::new(
                                                    px(0.0),
                                                    px(0.0),
                                                    accent_glow.opacity(0.45),
                                                )
                                                .blur_radius(px(14.0))
                                                .spread_radius(px(1.0))])
                                            })
                                            .child(
                                                gpui_base::Scrollbar::vertical(&scrollbar_state)
                                                    .id("document-scrollbar-vertical")
                                                    .mode(gpui_base::ScrollbarMode::Always)
                                                    .styles(|styles| {
                                                        styles
                                                            .thumb(|thumb| {
                                                                thumb.bg(rgb(theme.faint_text)
                                                                    .opacity(0.42))
                                                            })
                                                            .track_hover(|track| {
                                                                track.bg(accent_glow.opacity(0.12))
                                                            })
                                                            .thumb_hover(|thumb| {
                                                                thumb.bg(accent_color)
                                                            })
                                                            .thumb_active(|thumb| {
                                                                thumb.bg(accent_color)
                                                            })
                                                    })
                                                    .viewport_from_layout(),
                                            ),
                                    ),
                            ),
                    )
                    .into_any_element()
            })
    }
}
