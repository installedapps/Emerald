use super::*;

fn is_select_all(keystroke: &Keystroke) -> bool {
    keystroke.key == "a"
        && (keystroke.modifiers.control || keystroke.modifiers.platform)
        && !keystroke.modifiers.alt
        && !keystroke.modifiers.function
}

fn edit_rename_buffer(buffer: &mut String, keystroke: &Keystroke) {
    if keystroke.modifiers.control
        || keystroke.modifiers.alt
        || keystroke.modifiers.platform
        || keystroke.modifiers.function
    {
        return;
    }
    if keystroke.key == "backspace" {
        buffer.pop();
    } else if let Some(text) = &keystroke.key_char {
        buffer.push_str(text);
    }
}

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
#[path = "tests/input.rs"]
mod tests;

impl super::Emerald {
    pub(crate) fn handle_keystroke(&mut self, keystroke: &Keystroke, cx: &mut Context<Self>) {
        if self.interaction.graph_mode {
            self.interaction.context_menu = None;
            return;
        }
        if self.file_loading() {
            return;
        }
        if self.interaction.renaming {
            match keystroke.key.as_str() {
                "enter" => match self
                    .state
                    .rename_active_file(&self.interaction.rename_buffer)
                {
                    Ok(()) => {
                        self.interaction.file_load_error = None;
                        self.interaction.renaming = false;
                        self.interaction.rename_buffer.clear();
                    }
                    Err(error) => self.show_operation_error(error, cx),
                },
                "escape" => {
                    self.interaction.renaming = false;
                    self.interaction.rename_buffer.clear();
                }
                _ => edit_rename_buffer(&mut self.interaction.rename_buffer, keystroke),
            }
            self.blink_cursor.show();
            cx.notify();
            return;
        }
        if is_select_all(keystroke) && !self.interaction.graph_mode {
            self.interaction.context_menu = None;
            self.state.select_all();
            self.reveal_source(cx);
            self.blink_cursor.show();
            cx.notify();
            return;
        }
        if keystroke.modifiers.control
            || keystroke.modifiers.alt
            || keystroke.modifiers.platform
            || keystroke.modifiers.function
        {
            return;
        }

        if self.handle_slash_key(&keystroke.key, cx) {
            return;
        }

        match keystroke.key.as_str() {
            "backspace" => self.state.apply_edit_command(EditCommand::Backspace),
            "delete" => self.state.apply_edit_command(EditCommand::Delete),
            "enter" => self.state.apply_edit_command(EditCommand::Newline),
            "left" => self.state.apply_edit_command(EditCommand::MoveLeft {
                selecting: keystroke.modifiers.shift,
            }),
            "right" => self.state.apply_edit_command(EditCommand::MoveRight {
                selecting: keystroke.modifiers.shift,
            }),
            "up" => self.state.apply_edit_command(EditCommand::MoveUp {
                selecting: keystroke.modifiers.shift,
            }),
            "down" => self.state.apply_edit_command(EditCommand::MoveDown {
                selecting: keystroke.modifiers.shift,
            }),
            "home" => self.state.apply_edit_command(EditCommand::MoveLineStart {
                selecting: keystroke.modifiers.shift,
            }),
            "end" => self.state.apply_edit_command(EditCommand::MoveLineEnd {
                selecting: keystroke.modifiers.shift,
            }),
            "tab" => self
                .state
                .apply_edit_command(EditCommand::Insert("  ".to_string())),
            _ => {
                if let Some(key_char) = &keystroke.key_char {
                    self.state
                        .apply_edit_command(EditCommand::Insert(key_char.clone()));
                }
            }
        }

        self.refresh_slash_search(keystroke.key_char.as_deref() == Some("/"));
        self.blink_cursor.show();
        self.schedule_parse_refresh(cx);
        self.reveal_source(cx);
        cx.notify();
    }

    pub(super) fn byte_offset_for_document_position(
        &self,
        pointer_x: f32,
        pointer_y: f32,
    ) -> Option<usize> {
        super::source_geometry::source_offset_at(
            &self.interaction.source_geometry.borrow(),
            (pointer_x, pointer_y),
        )
    }

    pub(super) fn start_mouse_selection(
        &mut self,
        event: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.file_loading() {
            return;
        }
        self.interaction.context_menu = None;
        window.focus(&self.focus_handle, cx);
        cx.activate(true);
        cx.stop_propagation();
        self.interaction.is_mouse_selecting = true;
        self.interaction.editing_revision = self.interaction.editing_revision.wrapping_add(1);
        let pointer = (f32::from(event.position.x), f32::from(event.position.y));
        if self.interaction.source_revealed {
            if let Some(cursor) = self.byte_offset_for_document_position(pointer.0, pointer.1) {
                if event.modifiers.shift {
                    self.state.extend_mouse_selection(cursor);
                } else {
                    self.state.begin_mouse_selection(cursor);
                }
            }
        } else {
            // Resolve the anchor only after the newly revealed source has been laid out.
            self.interaction.pending_source_click = Some((pointer, event.modifiers.shift));
            self.interaction.source_revealed = true;
        }
        self.blink_cursor.show();
        cx.notify();
    }

    pub(super) fn update_mouse_selection_from_drag(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.interaction.is_mouse_selecting || event.pressed_button != Some(MouseButton::Left) {
            return;
        }

        let Some(cursor) = self.byte_offset_for_document_position(
            f32::from(event.position.x),
            f32::from(event.position.y),
        ) else {
            return;
        };
        self.state.update_mouse_selection(cursor);
        self.blink_cursor.show();
        cx.notify();
    }

    pub(super) fn finish_mouse_selection(
        &mut self,
        event: &MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if !self.interaction.is_mouse_selecting {
            return;
        }

        if let Some(cursor) = self.byte_offset_for_document_position(
            f32::from(event.position.x),
            f32::from(event.position.y),
        ) {
            self.state.update_mouse_selection(cursor);
        }
        self.interaction.is_mouse_selecting = false;
        self.state.end_mouse_selection();
        self.blink_cursor.show();
        self.reveal_source(cx);
        cx.notify();
    }
}
