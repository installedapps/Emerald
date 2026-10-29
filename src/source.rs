use super::source_geometry::SourceRowGeometry;
use super::*;
use emerald::preview::PreparedPreview;
use gpui::{canvas, fill, point};

impl super::Emerald {
    pub(super) fn render_source_line(
        line: source_layout::SourceRowLayout,
        theme: emerald::theme::EmeraldTheme,
        cursor: usize,
        selection: Option<&emerald::Selection>,
        cursor_visible: bool,
        spelling_errors: Arc<Vec<std::ops::Range<usize>>>,
        geometry: Rc<RefCell<Vec<SourceRowGeometry>>>,
    ) -> AnyElement {
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
                        for error in spelling_errors.iter().filter(|error| {
                            error.start < line.range.end && error.end > line.range.start
                        }) {
                            let start = error.start.max(line.range.start) - line_start;
                            let end = error.end.min(line.range.end) - line_start;
                            let left = row.x_for_offset(start);
                            let right = row.x_for_offset(end);
                            if right <= left {
                                continue;
                            }
                            let mut squiggle = gpui::PathBuilder::stroke(px(1.2));
                            squiggle.move_to(
                                bounds.origin + point(px(left), px(SOURCE_LINE_HEIGHT - 3.0)),
                            );
                            let mut x = left;
                            let mut high = true;
                            while x < right {
                                x = (x + 3.0).min(right);
                                squiggle.line_to(
                                    bounds.origin
                                        + point(
                                            px(x),
                                            px(SOURCE_LINE_HEIGHT - if high { 4.5 } else { 1.5 }),
                                        ),
                                );
                                high = !high;
                            }
                            if let Ok(path) = squiggle.build() {
                                window.paint_path(path, rgb(0xe05252));
                            }
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
        let theme = self.theme();
        let cursor = self.state.cursor();
        let selection = self.state.selection();
        let file = self.state.active_file().to_path_buf();
        let content_changed = self
            .interaction
            .source_lines_cache
            .as_ref()
            .is_none_or(|cache| cache.text_revision != text_revision || cache.file != file);
        let content_changed = content_changed
            || self
                .interaction
                .source_lines_cache
                .as_ref()
                .is_some_and(|cache| cache.theme_text != theme.text);
        let width_changed = self
            .interaction
            .source_lines_cache
            .as_ref()
            .is_some_and(|cache| cache.wrap_width != wrap_width);
        if content_changed {
            let text: Arc<str> = self.state.text().into();
            let ranges = Arc::new(emerald::ui::source_line_ranges(&text));
            let spelling_errors = Arc::new(super::spellcheck::misspellings(&text));
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
                theme_text: theme.text,
                text_revision,
                file,
                text,
                ranges,
                spelling_errors,
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
        let spelling_errors = cache.spelling_errors.clone();
        let layouts = cache.layouts.clone();
        let cursor_visible = self.blink_cursor.visible();
        let geometry = self.interaction.source_geometry.clone();
        geometry.borrow_mut().clear();
        let width_entity = entity.clone();
        let list = list(self.source_list_state.clone(), move |index, window, _| {
            let range = ranges[index].clone();
            let rows = layouts.borrow_mut().get_or_prepare(
                index,
                &text[range.clone()],
                range.start,
                wrap_width,
                theme.text,
                window,
            );
            div()
                .flex()
                .flex_col()
                .w_full()
                .children(rows.iter().map(|row| {
                    Self::render_source_line(
                        row.clone(),
                        theme,
                        cursor,
                        selection.as_ref(),
                        cursor_visible,
                        spelling_errors.clone(),
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
                .inset_0()
                .size_full(),
            )
            .child(list)
            .child(
                canvas(
                    |_, _, _| (),
                    move |bounds, _, window, cx| {
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

                            if layout_current && view.interaction.source_cursor_reveal_pending {
                                if view.interaction.source_cursor_reveal_waiting_for_layout {
                                    view.interaction.source_cursor_reveal_waiting_for_layout =
                                        false;
                                    cx.notify();
                                } else {
                                    let cursor = view.state.cursor();
                                    let text_len = view
                                        .interaction
                                        .source_lines_cache
                                        .as_ref()
                                        .map_or(0, |cache| cache.text.len());

                                    if cursor == text_len {
                                        view.source_list_state.scroll_to_end();
                                        view.interaction.source_cursor_reveal_pending = false;
                                        cx.notify();
                                    } else {
                                        let cursor_row = view
                                            .interaction
                                            .source_geometry
                                            .borrow()
                                            .iter()
                                            .filter(|row| {
                                                let local = cursor.saturating_sub(row.start);
                                                cursor >= row.start
                                                    && row
                                                        .stops
                                                        .last()
                                                        .is_some_and(|(end, _)| local <= *end)
                                            })
                                            .max_by_key(|row| row.start)
                                            .map(|row| (row.origin.1, row.height));

                                        if let Some((row_top, row_height)) = cursor_row {
                                            let viewport_top = f32::from(bounds.origin.y);
                                            let viewport_bottom =
                                                viewport_top + f32::from(bounds.size.height);
                                            let row_bottom = row_top + row_height;
                                            let scroll_delta = if row_top < viewport_top {
                                                row_top - viewport_top
                                            } else if row_bottom > viewport_bottom {
                                                row_bottom - viewport_bottom
                                            } else {
                                                0.0
                                            };
                                            if scroll_delta != 0.0 {
                                                view.source_list_state.scroll_by(px(scroll_delta));
                                                view.interaction
                                                    .source_cursor_reveal_waiting_for_layout = true;
                                            } else {
                                                view.interaction.source_cursor_reveal_pending =
                                                    false;
                                            }
                                        } else if let Some(index) = view
                                            .interaction
                                            .source_lines_cache
                                            .as_ref()
                                            .and_then(|cache| {
                                                cache
                                                    .ranges
                                                    .iter()
                                                    .position(|range| {
                                                        range.start <= cursor && cursor < range.end
                                                    })
                                                    .or_else(|| {
                                                        cache.ranges.iter().position(|range| {
                                                            range.is_empty()
                                                                && range.start == cursor
                                                        })
                                                    })
                                                    .or_else(|| {
                                                        cache.ranges.last().and_then(|range| {
                                                            (range.end == cursor).then_some(
                                                                cache
                                                                    .ranges
                                                                    .len()
                                                                    .saturating_sub(1),
                                                            )
                                                        })
                                                    })
                                            })
                                        {
                                            view.source_list_state.scroll_to_reveal_item(index);
                                            view.interaction
                                                .source_cursor_reveal_waiting_for_layout = true;
                                        } else {
                                            view.interaction.source_cursor_reveal_pending = false;
                                        }

                                        cx.notify();
                                    }
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
                .inset_0()
                .size_full(),
            )
            .into_any_element()
    }

    pub(super) fn render_text_parts(
        text: &str,
        prepared: &PreparedPreview,
        document_path: &std::path::Path,
        theme: emerald::theme::EmeraldTheme,
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
                let element = div().min_w(px(0.0)).max_w_full().whitespace_normal().child(
                    gpui::SharedString::new(preserve_repeated_spaces(&part.text)),
                );
                if let Some(target) = &part.target {
                    let target = target.clone();
                    let entity = entity.clone();
                    element
                        .text_color(rgb(theme.accent))
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

fn preserve_repeated_spaces(text: &str) -> String {
    let mut rendered = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(character) = chars.next() {
        if character != ' ' {
            rendered.push(character);
            continue;
        }
        let mut count = 1;
        while chars.peek() == Some(&' ') {
            chars.next();
            count += 1;
        }
        if count == 1 {
            rendered.push(' ');
        } else {
            rendered.extend(std::iter::repeat_n('\u{00a0}', count));
            rendered.push('\u{200b}');
        }
    }
    rendered
}
