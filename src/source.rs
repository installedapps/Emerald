use super::source_geometry::SourceRowGeometry;
use super::*;
use emerald::preview::PreparedPreview;
use gpui::{canvas, fill, point};

impl super::Emerald {
    pub(super) fn render_source_line(
        line: source_layout::SourceRowLayout,
        cursor: usize,
        selection: Option<&emerald::Selection>,
        cursor_visible: bool,
        geometry: Rc<RefCell<Vec<SourceRowGeometry>>>,
    ) -> AnyElement {
        let theme = EVERFOREST_DARK;
        let line_start = line.range.start;
        let selected = selection.and_then(|selection| {
            let start = selection.start.max(line.range.start);
            let end = selection.end.min(line.range.end);
            (start < end).then(|| start - line_start..end - line_start)
        });
        let newline_selected = line.newline
            && selection.is_some_and(|selection| {
                selection.start <= line.range.end && line.range.end < selection.end
            });
        let cursor = (cursor >= line.range.start
            && (cursor < line.range.end || (line.last && cursor == line.range.end)))
            .then(|| cursor - line_start);
        let shaped = line.shaped;
        let stops = line.stops;
        div()
            .id(("source-line", line_start))
            .h(px(SOURCE_LINE_HEIGHT))
            .w_full()
            .min_w(px(0.0))
            .child(
                canvas(
                    move |bounds, _, _| {
                        let row = SourceRowGeometry {
                            start: line_start,
                            origin: (f32::from(bounds.origin.x), f32::from(bounds.origin.y)),
                            height: SOURCE_LINE_HEIGHT,
                            stops,
                        };
                        let mut rows = geometry.borrow_mut();
                        rows.retain(|old| old.start != line_start);
                        rows.push(row.clone());
                        row
                    },
                    move |bounds, row, window, cx| {
                        let highlight = |start: f32, end: f32, window: &mut Window| {
                            window.paint_quad(fill(
                                Bounds::new(
                                    bounds.origin + point(px(start), px(0.0)),
                                    size(px(end - start), px(SOURCE_LINE_HEIGHT)),
                                ),
                                rgb(theme.selection),
                            ));
                        };
                        if let Some((start, end)) =
                            selected.and_then(|range| row.selection_span(range))
                        {
                            highlight(start, end, window);
                        }
                        if newline_selected {
                            let end = f32::from(shaped.width);
                            highlight(end, end + 8.0, window);
                        }
                        if let Err(error) = shaped.paint(
                            bounds.origin,
                            px(SOURCE_LINE_HEIGHT),
                            gpui::TextAlign::Left,
                            None,
                            window,
                            cx,
                        ) {
                            tracing::warn!(%error, "could not paint source line");
                        }
                        if let Some(cursor) = cursor.filter(|_| cursor_visible) {
                            let style = cursor_style(theme);
                            window.paint_quad(fill(
                                Bounds::new(
                                    bounds.origin + point(px(row.x_for_offset(cursor)), px(3.0)),
                                    size(px(style.width), px(22.0)),
                                ),
                                rgb(style.color),
                            ));
                        }
                    },
                )
                .size_full(),
            )
            .into_any_element()
    }

    pub(super) fn render_source(
        &mut self,
        entity: gpui::Entity<Emerald>,
        window: &mut Window,
    ) -> AnyElement {
        let wrap_width = if self.interaction.source_wrap_width > 0.0 {
            self.interaction.source_wrap_width
        } else {
            f32::from(window.viewport_size().width).max(1.0)
        };
        let text_revision = self.state.text_revision();
        let cursor = self.state.cursor();
        let selection = self.state.selection();
        let file = self.state.active_file().to_path_buf();
        let content_changed = self
            .interaction
            .source_lines_cache
            .as_ref()
            .is_none_or(|cache| cache.text_revision != text_revision || cache.file != file);
        let width_changed = self
            .interaction
            .source_lines_cache
            .as_ref()
            .is_some_and(|cache| cache.wrap_width != wrap_width);
        if content_changed {
            let text: Arc<str> = self.state.text().into();
            let ranges = Arc::new(emerald::ui::source_line_ranges(&text));
            let file_changed = self
                .interaction
                .source_lines_cache
                .as_ref()
                .is_none_or(|cache| cache.file != file);
            let scroll = self.source_list_state.logical_scroll_top();
            self.source_list_state.reset(ranges.len());
            if !file_changed {
                self.source_list_state.scroll_to(scroll);
            }
            self.interaction.source_lines_cache = Some(SourceLinesCache {
                wrap_width,
                text_revision,
                file,
                text,
                ranges,
                layouts: Rc::default(),
            });
        } else if width_changed {
            let cache = self.interaction.source_lines_cache.as_mut().unwrap();
            cache.wrap_width = wrap_width;
            cache.layouts = Rc::default();
            self.source_list_state
                .splice(0..cache.ranges.len(), cache.ranges.len());
        }
        let cache = self.interaction.source_lines_cache.as_ref().unwrap();
        let text = cache.text.clone();
        let ranges = cache.ranges.clone();
        let layouts = cache.layouts.clone();
        let cursor_visible = self.blink_cursor.visible();
        let geometry = self.interaction.source_geometry.clone();
        let frame_geometry = geometry.clone();
        let width_entity = entity.clone();
        let list = list(self.source_list_state.clone(), move |index, window, _| {
            let range = ranges[index].clone();
            let rows = layouts.borrow_mut().get_or_prepare(
                index,
                &text[range.clone()],
                range.start,
                wrap_width,
                window,
            );
            div()
                .flex()
                .flex_col()
                .w_full()
                .children(rows.iter().map(|row| {
                    Self::render_source_line(
                        row.clone(),
                        cursor,
                        selection.as_ref(),
                        cursor_visible,
                        geometry.clone(),
                    )
                }))
                .into_any_element()
        })
        // Size from the viewport so GPUI only shapes rows at their final width.
        .with_sizing_behavior(gpui::ListSizingBehavior::Auto)
        .w_full()
        .h_full();

        div()
            .relative()
            .size_full()
            .child(
                canvas(
                    move |bounds, _, cx| {
                        frame_geometry.borrow_mut().clear();
                        let width = (f32::from(bounds.size.width) - 10.0).max(1.0);
                        width_entity.update(cx, |view, cx| {
                            if view.interaction.source_wrap_width != width {
                                view.interaction.source_wrap_width = width;
                                cx.notify();
                            }
                        });
                    },
                    |_, _, _, _| (),
                )
                .absolute()
                .size_full(),
            )
            .child(list)
            .child(
                canvas(
                    |_, _, _| (),
                    move |_, _, window, cx| {
                        entity.update(cx, |view, cx| {
                            let layout_current = view
                                .interaction
                                .source_lines_cache
                                .as_ref()
                                .is_some_and(|cache| {
                                    cache.wrap_width == view.interaction.source_wrap_width
                                });
                            if let Some((pointer, shift)) = layout_current
                                .then(|| view.interaction.pending_source_click.take())
                                .flatten()
                            {
                                if let Some(cursor) =
                                    view.byte_offset_for_document_position(pointer.0, pointer.1)
                                {
                                    if shift {
                                        view.state.extend_mouse_selection(cursor);
                                    } else {
                                        view.state.begin_mouse_selection(cursor);
                                    }
                                    if !view.interaction.is_mouse_selecting {
                                        view.state.end_mouse_selection();
                                    }
                                    cx.notify();
                                }
                            }
                        });
                        let moving = entity.downgrade();
                        window.on_mouse_event(move |event: &MouseMoveEvent, phase, window, cx| {
                            if phase == gpui::DispatchPhase::Bubble {
                                let _ = moving.update(cx, |view, cx| {
                                    view.update_mouse_selection_from_drag(event, window, cx);
                                });
                            }
                        });
                        let releasing = entity.downgrade();
                        window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
                            if phase == gpui::DispatchPhase::Bubble
                                && event.button == MouseButton::Left
                            {
                                let _ = releasing.update(cx, |view, cx| {
                                    view.finish_mouse_selection(event, window, cx);
                                });
                            }
                        });
                    },
                )
                .absolute()
                .size_full(),
            )
            .into_any_element()
    }

    pub(super) fn render_text_parts(
        text: &str,
        prepared: &PreparedPreview,
        document_path: &std::path::Path,
        entity: gpui::Entity<Emerald>,
    ) -> Vec<AnyElement> {
        prepared
            .inline(text)
            .iter()
            .map(|part| {
                if let Some(image) = &part.image {
                    let path = document_path
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new("."))
                        .join(image.target.as_ref());
                    let label = format!("Image unavailable: {}", part.text);
                    return gpui::img(path)
                        .w(px(image.width.unwrap_or(24) as f32))
                        .max_w_full()
                        .with_fallback(move || div().child(label.clone()).into_any_element())
                        .into_any_element();
                }
                let element = div()
                    .min_w(px(0.0))
                    .max_w_full()
                    .whitespace_normal()
                    .child(gpui::SharedString::new(part.text.clone()));
                if let Some(target) = &part.target {
                    let target = target.clone();
                    let entity = entity.clone();
                    element
                        .text_color(rgb(EVERFOREST_DARK.accent))
                        .underline()
                        .cursor_pointer()
                        .on_mouse_down(MouseButton::Left, move |_, window, cx| {
                            cx.stop_propagation();
                            entity.update(cx, |view, cx| {
                                view.open_link(target.to_string(), window, cx)
                            });
                        })
                        .into_any_element()
                } else {
                    element.into_any_element()
                }
            })
            .collect()
    }
}
