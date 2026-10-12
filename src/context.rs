use super::*;
use emerald::theme::ThemeId;

impl super::Emerald {
    pub(super) fn dismiss_context_menu(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.interaction.context_menu.take().is_some() {
            cx.notify();
        }
    }

    fn keep_context_menu_open(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
    }

    pub(super) fn show_sidebar_context_menu(
        &mut self,
        file: PathBuf,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        self.interaction.context_menu = Some(ContextMenu::Sidebar {
            position: event.position,
            file,
        });
        cx.notify();
    }

    pub(super) fn show_editor_context_menu(
        &mut self,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let spelling = if self.interaction.source_revealed {
            let pointer = (f32::from(event.position.x), f32::from(event.position.y));
            self.byte_offset_for_document_position(pointer.0, pointer.1)
                .and_then(|offset| {
                    self.interaction
                        .source_lines_cache
                        .as_ref()?
                        .spelling_errors
                        .iter()
                        .find(|range| range.start <= offset && offset < range.end)
                        .cloned()
                })
                .and_then(|range| {
                    self.state
                        .text()
                        .get(range.clone())
                        .map(|word| (range, word.to_owned()))
                })
        } else {
            None
        };
        self.interaction.context_menu = Some(spelling.map_or_else(
            || ContextMenu::Editor {
                position: event.position,
            },
            |(range, word)| ContextMenu::Spelling {
                position: event.position,
                range,
                suggestions: super::spellcheck::suggestions(&word),
                word,
            },
        ));
        cx.notify();
    }

    fn context_replace_misspelling(
        &mut self,
        range: std::ops::Range<usize>,
        replacement: String,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.set_cursor(range.start, false);
        self.state.set_cursor(range.end, true);
        self.state.insert_text(&replacement);
        self.interaction.context_menu = None;
        self.schedule_parse_refresh(cx);
        self.reveal_source(cx);
        cx.notify();
    }

    pub(super) fn context_copy(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(text) = self.state.selected_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text.to_string()));
        }
        self.interaction.context_menu = None;
        cx.notify();
    }

    pub(super) fn context_paste(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            self.state.insert_text(&text);
            self.schedule_parse_refresh(cx);
        }
        self.interaction.context_menu = None;
        cx.notify();
    }

    pub(super) fn context_select_all(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.state.select_all();
        self.reveal_source(cx);
        self.interaction.context_menu = None;
        cx.notify();
    }

    pub(super) fn context_rename(
        &mut self,
        file: PathBuf,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.begin_rename(file, window, cx);
    }

    pub(super) fn render_context_menu(
        &self,
        menu: ContextMenu,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (position, items): (gpui::Point<gpui::Pixels>, Vec<AnyElement>) = match menu {
            ContextMenu::Slash(search) => (
                self.slash_menu_position(&search),
                self.slash_menu_items(&search, cx),
            ),
            ContextMenu::Editor { position } => (
                position,
                vec![
                    div()
                        .p_2()
                        .child("Copy")
                        .on_mouse_down(MouseButton::Left, cx.listener(Self::context_copy))
                        .into_any_element(),
                    div()
                        .p_2()
                        .child("Paste")
                        .on_mouse_down(MouseButton::Left, cx.listener(Self::context_paste))
                        .into_any_element(),
                    div()
                        .p_2()
                        .child("Select all")
                        .on_mouse_down(MouseButton::Left, cx.listener(Self::context_select_all))
                        .into_any_element(),
                ],
            ),
            ContextMenu::Sidebar { position, file } => (
                position,
                vec![div()
                    .p_2()
                    .child("Rename")
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |view, event, window, cx| {
                            view.context_rename(file.clone(), event, window, cx);
                        }),
                    )
                    .into_any_element()],
            ),
            ContextMenu::Spelling {
                position,
                range,
                word,
                suggestions,
            } => {
                let mut items = vec![div()
                    .p_2()
                    .text_xs()
                    .text_color(rgb(self.theme().muted_text))
                    .child(format!("Suggestions for ‘{word}’"))
                    .into_any_element()];
                if suggestions.is_empty() {
                    items.push(
                        div()
                            .p_2()
                            .text_xs()
                            .text_color(rgb(self.theme().muted_text))
                            .child("No suggestions")
                            .into_any_element(),
                    );
                } else {
                    items.extend(suggestions.into_iter().map(|suggestion| {
                        let range = range.clone();
                        div()
                            .w_full()
                            .p_2()
                            .cursor_pointer()
                            .hover(|el| el.bg(rgb(self.theme().hover)))
                            .child(suggestion.clone())
                            .on_mouse_down(
                                MouseButton::Left,
                                cx.listener(move |view, event, window, cx| {
                                    view.context_replace_misspelling(
                                        range.clone(),
                                        suggestion.clone(),
                                        event,
                                        window,
                                        cx,
                                    );
                                }),
                            )
                            .into_any_element()
                    }));
                }
                (position, items)
            }
            ContextMenu::Themes { position } => {
                let theme = self.theme();
                let favorites = self.theme_preferences.favorites.clone();
                let shown = ThemeId::ALL
                    .into_iter()
                    .filter(|id| !self.interaction.favorites_only || favorites.contains(id))
                    .collect::<Vec<_>>();
                let filter = div()
                    .id("theme-favorites-filter")
                    .role(gpui::Role::Button)
                    .aria_label(if self.interaction.favorites_only {
                        "Show all themes"
                    } else {
                        "Show favorite themes only"
                    })
                    .tab_stop(true)
                    .focus_visible(|el| el.bg(rgb(theme.hover)).text_color(rgb(theme.text)))
                    .w_full()
                    .p_2()
                    .text_xs()
                    .text_color(rgb(theme.muted_text))
                    .cursor_pointer()
                    .hover(|el| el.bg(rgb(theme.hover)))
                    .child(if self.interaction.favorites_only {
                        "★ Favorites only · Show all"
                    } else {
                        "★ Favorites only"
                    })
                    .on_click(cx.listener(|view, _, _, cx| {
                        view.interaction.favorites_only = !view.interaction.favorites_only;
                        cx.notify();
                    }))
                    .into_any_element();
                let mut items = vec![filter];
                if shown.is_empty() {
                    items.push(
                        div()
                            .px_2()
                            .py_2()
                            .text_xs()
                            .text_color(rgb(theme.faint_text))
                            .child("No favorite themes yet")
                            .into_any_element(),
                    );
                }
                items.extend(shown.into_iter().map(|id| {
                    let selected = self.theme_preferences.selected == id;
                    let favorite = favorites.contains(&id);
                    div()
                        .id(gpui::SharedString::from(format!("theme-{}", id.id())))
                        .role(gpui::Role::Button)
                        .aria_label(format!(
                            "{} theme{}",
                            id.name(),
                            if selected { ", selected" } else { "" }
                        ))
                        .tab_stop(true)
                        .focus_visible(|el| el.border_color(rgb(theme.accent)))
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap_3()
                        .px_2()
                        .py_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .text_color(rgb(if selected {
                            theme.text
                        } else {
                            theme.muted_text
                        }))
                        .when(selected, |el| el.bg(rgb(theme.selection)))
                        .hover(|el| el.bg(rgb(theme.hover)).text_color(rgb(theme.text)))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .size(px(10.0))
                                        .rounded_full()
                                        .bg(rgb(id.theme().accent)),
                                )
                                .child(id.name()),
                        )
                        .child(
                            div()
                                .id(gpui::SharedString::from(format!("favorite-{}", id.id())))
                                .role(gpui::Role::Button)
                                .aria_label(if favorite {
                                    "Remove favorite"
                                } else {
                                    "Add favorite"
                                })
                                .tab_stop(true)
                                .focus_visible(|el| el.text_color(rgb(theme.text)))
                                .px_1()
                                .text_color(rgb(if favorite {
                                    id.theme().accent
                                } else {
                                    theme.faint_text
                                }))
                                .child(if favorite { "★" } else { "☆" })
                                .on_click(cx.listener(move |view, _, _, cx| {
                                    cx.stop_propagation();
                                    view.toggle_theme_favorite(id, cx);
                                })),
                        )
                        .on_click(cx.listener(move |view, _, _, cx| view.select_theme(id, cx)))
                        .into_any_element()
                }));
                (position, items)
            }
        };

        gpui::deferred(
            gpui::anchored().position(position).snap_to_window().child(
                div()
                    .p_1()
                    .bg(rgb(self.theme().menu))
                    .border_1()
                    .border_color(rgb(self.theme().panel_border))
                    .text_color(rgb(self.theme().text))
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::keep_context_menu_open))
                    .children(items),
            ),
        )
        .into_any_element()
    }
}
