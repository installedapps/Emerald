use super::*;

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
        self.interaction.context_menu = Some(ContextMenu::Editor {
            position: event.position,
        });
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
        };

        gpui::deferred(
            gpui::anchored().position(position).snap_to_window().child(
                div()
                    .p_1()
                    .bg(rgb(EVERFOREST_DARK.panel))
                    .border_1()
                    .border_color(rgb(EVERFOREST_DARK.panel_border))
                    .text_color(rgb(EVERFOREST_DARK.text))
                    .on_mouse_down(MouseButton::Left, cx.listener(Self::keep_context_menu_open))
                    .children(items),
            ),
        )
        .into_any_element()
    }
}
