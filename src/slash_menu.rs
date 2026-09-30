use super::*;
use emerald::slash_commands::{SlashSearch, COMMANDS};

impl Emerald {
    pub(super) fn handle_slash_key(&mut self, key: &str, cx: &mut Context<Self>) -> bool {
        let Some(ContextMenu::Slash(search)) = &mut self.interaction.context_menu else {
            return false;
        };
        match key {
            "up" | "down" => search.navigate(key == "up"),
            "enter" | "tab" => {
                if let Some(&index) = search.matches.get(search.selected) {
                    self.insert_slash_command(index, cx);
                }
            }
            "escape" => {
                self.interaction.context_menu = None;
                self.reveal_source(cx);
            }
            "backspace" => return false,
            key if key.chars().count() == 1 && key.chars().all(char::is_alphabetic) => {
                return false
            }
            _ => {
                self.interaction.context_menu = None;
                return false;
            }
        }
        cx.notify();
        true
    }

    pub(super) fn refresh_slash_search(&mut self, typed_slash: bool) {
        if typed_slash || matches!(self.interaction.context_menu, Some(ContextMenu::Slash(_))) {
            self.interaction.context_menu = if self.state.selection().is_none() {
                SlashSearch::at(self.state.text(), self.state.cursor()).map(ContextMenu::Slash)
            } else {
                None
            };
        }
    }

    pub(super) fn insert_slash_command(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(ContextMenu::Slash(search)) = self.interaction.context_menu.take() else {
            return;
        };
        // Validate the trigger again so stale mouse events cannot replace unrelated text.
        if self.file_loading()
            || self.interaction.graph_mode
            || self.interaction.renaming
            || self.state.selection().is_some()
            || SlashSearch::at(self.state.text(), self.state.cursor())
                .is_none_or(|current| current.range != search.range)
        {
            return;
        }
        let Some(command) = COMMANDS.get(index) else {
            return;
        };
        let (text, placeholder) = search.replacement(self.state.text(), *command);
        self.state.set_cursor(search.range.start, false);
        self.state.set_cursor(search.range.end, true);
        self.state.insert_text(&text);
        self.state.set_cursor(placeholder.start, false);
        self.state.set_cursor(placeholder.end, true);
        self.blink_cursor.show();
        self.schedule_parse_refresh(cx);
        self.reveal_source(cx);
        cx.notify();
    }

    pub(super) fn slash_menu_items(
        &self,
        search: &SlashSearch,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        if search.matches.is_empty() {
            return vec![div().p_2().child("No matching commands").into_any_element()];
        }
        search
            .matches
            .iter()
            .enumerate()
            .map(|(row, &index)| {
                div()
                    .p_2()
                    .cursor_pointer()
                    .when(row == search.selected, |item| {
                        item.bg(rgb(self.theme().selection))
                    })
                    .hover(|style| style.bg(rgb(self.theme().selection)))
                    .child(COMMANDS[index].label)
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |view, _, _, cx| {
                            cx.stop_propagation();
                            view.insert_slash_command(index, cx);
                        }),
                    )
                    .into_any_element()
            })
            .collect()
    }

    pub(super) fn slash_menu_position(&self, search: &SlashSearch) -> gpui::Point<gpui::Pixels> {
        let rows = self.interaction.source_geometry.borrow();
        let row = rows
            .iter()
            .filter(|row| row.start <= search.range.start)
            .max_by_key(|row| row.start);
        row.map_or(gpui::point(px(280.0), px(110.0)), |row| {
            gpui::point(
                px(row.origin.0 + row.x_for_offset(search.range.start - row.start)),
                px(row.origin.1 + row.height),
            )
        })
    }
}
