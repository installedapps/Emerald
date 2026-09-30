use super::*;

impl super::Emerald {
    pub(super) fn open_link(
        &mut self,
        target: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = self.state.workspace_root().join(target);
        let path = match self.state.ensure_linked_file(path) {
            Ok(path) => path,
            Err(error) => {
                self.show_operation_error(error, cx);
                return;
            }
        };
        if path == self.state.active_file() {
            return;
        }
        self.interaction.context_menu = None;
        self.interaction.source_revealed = false;
        self.source_list_state.scroll_to(ListOffset {
            item_ix: 0,
            offset_in_item: px(0.0),
        });
        self.start_file_load(path, cx);
        window.focus(&self.focus_handle, cx);
        cx.activate(true);
    }

    pub(super) fn toggle_graph_settings(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.interaction.graph_settings = !self.interaction.graph_settings;
        cx.notify();
    }

    pub(super) fn graph_scroll(
        &mut self,
        event: &gpui::ScrollWheelEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let delta = match event.delta {
            gpui::ScrollDelta::Pixels(point) => f32::from(point.y),
            gpui::ScrollDelta::Lines(point) => point.y * 48.0,
        };
        self.interaction.graph_zoom =
            (self.interaction.graph_zoom * (1.0 - delta * 0.001)).clamp(0.45, 2.5);
        cx.notify();
    }

    pub(super) fn graph_pan_start(
        &mut self,
        event: &MouseDownEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Middle {
            self.interaction.graph_drag_origin = Some(self.interaction.graph_offset);
            self.interaction.graph_drag_start =
                gpui::point(f32::from(event.position.x), f32::from(event.position.y));
        }
    }

    pub(super) fn graph_pan_move(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if event.pressed_button == Some(MouseButton::Middle) {
            if let Some(origin) = self.interaction.graph_drag_origin {
                self.interaction.graph_offset = gpui::point(
                    origin.x + f32::from(event.position.x) - self.interaction.graph_drag_start.x,
                    origin.y + f32::from(event.position.y) - self.interaction.graph_drag_start.y,
                );
                cx.notify();
            }
        }
    }

    pub(super) fn graph_pan_end(
        &mut self,
        event: &gpui::MouseUpEvent,
        _: &mut Window,
        _: &mut Context<Self>,
    ) {
        if event.button == MouseButton::Middle {
            self.interaction.graph_drag_origin = None;
        }
    }

    pub(super) fn toggle_graph(
        &mut self,
        _: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.interaction.context_menu = None;
        self.interaction.graph_mode = !self.interaction.graph_mode;
        self.interaction.graph_hovered = None;
        cx.notify();
    }

    pub(super) fn open_graph_node(
        &mut self,
        path: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let path = match self.state.ensure_linked_file(&path) {
            Ok(path) => path,
            Err(error) => {
                self.show_operation_error(error, cx);
                return;
            }
        };
        self.interaction.graph_mode = false;
        self.start_file_load(path, cx);
        window.focus(&self.focus_handle, cx);
        cx.activate(true);
    }

    pub(super) fn show_operation_error(&mut self, error: impl ToString, cx: &mut Context<Self>) {
        let message = error.to_string();
        tracing::error!(error = %message, "workspace operation failed");
        self.interaction.file_load_error = Some(message);
        cx.notify();
    }

    pub(super) fn select_sidebar_file(
        &mut self,
        path: PathBuf,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if path == self.state.active_file() {
            return;
        }
        self.start_file_load(path, cx);
        self.source_list_state.scroll_to(ListOffset {
            item_ix: 0,
            offset_in_item: px(0.0),
        });
        self.interaction.context_menu = None;
        self.interaction.source_revealed = NoteSwitchIntent::ExistingFile.reveals_source();
        window.focus(&self.focus_handle, cx);
        cx.activate(true);
        self.blink_cursor.show();
        cx.notify();
    }

    fn start_file_load(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.interaction.context_menu = None;
        let workspace = self.state.workspace_root().to_path_buf();
        let current_file = self.state.active_file().to_path_buf();
        let current_text = self.state.text().to_string();
        let dirty = self.state.is_dirty();
        self.interaction.file_switch_revision =
            self.interaction.file_switch_revision.wrapping_add(1);
        let revision = self.interaction.file_switch_revision;
        self.interaction.file_loading = true;
        self.interaction.file_load_error = None;
        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let workspace = emerald::workspace::Workspace::open(&workspace)?;
                    let destination = workspace.resolve_existing(&path)?;
                    if dirty {
                        workspace.write_file(&current_file, &current_text)?;
                    }
                    let text = std::fs::read_to_string(&destination)?;
                    let parsed = emerald::parser::ParsedDocument::from_source(&text);
                    let snapshot =
                        emerald::rendered::RenderSnapshot::from_parsed(&text, parsed.clone());
                    Ok::<_, anyhow::Error>((destination, text, parsed, snapshot))
                })
                .await;
            let _ = view.update(cx, |view, cx| {
                if view.interaction.file_switch_revision != revision {
                    return;
                }
                view.interaction.file_loading = false;
                match result {
                    Ok((active_file, text, parsed, snapshot)) => {
                        view.state
                            .apply_loaded_file(active_file.clone(), text, parsed);
                        view.render_cache
                            .insert(active_file, view.state.text(), snapshot);
                        cx.notify();
                    }
                    Err(error) => {
                        tracing::error!(error = %error, "failed to load document");
                        view.interaction.file_load_error = Some(error.to_string());
                        cx.notify();
                    }
                }
            });
        })
        .detach();
    }

    pub(super) fn create_new_file(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.state.create_file("untitled") {
            self.show_operation_error(error, cx);
            window.focus(&self.focus_handle, cx);
            return;
        }
        self.interaction.file_load_error = None;
        self.source_list_state.scroll_to(ListOffset {
            item_ix: 0,
            offset_in_item: px(0.0),
        });
        self.interaction.context_menu = None;
        self.interaction.source_revealed = NoteSwitchIntent::NewFile.reveals_source();
        if self.interaction.source_revealed {
            self.reveal_source(cx);
        }
        window.focus(&self.focus_handle, cx);
        cx.activate(true);
        self.blink_cursor.show();
        cx.notify();
    }

    pub(super) fn save_current_file(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Err(error) = self.state.save() {
            self.show_operation_error(error, cx);
            window.focus(&self.focus_handle, cx);
            return;
        }
        self.interaction.file_load_error = None;
        window.focus(&self.focus_handle, cx);
        cx.activate(true);
        self.blink_cursor.show();
        cx.notify();
    }

    pub(super) fn open_file(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let receiver = cx.prompt_for_paths(PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some("Open AsciiDoc file".into()),
        });
        window.focus(&self.focus_handle, cx);
        cx.activate(true);

        cx.spawn(async move |view, cx| {
            let paths = match receiver.await {
                Ok(Ok(Some(paths))) => paths,
                Ok(Ok(None)) => return,
                Ok(Err(error)) => {
                    tracing::warn!(error = %error, "opening a file failed");
                    return;
                }
                Err(error) => {
                    tracing::warn!(error = %error, "opening a file was cancelled");
                    return;
                }
            };
            let Some(path) = paths.into_iter().next() else {
                return;
            };
            let result = view.update(cx, |view, cx| {
                if path != view.state.active_file() {
                    view.start_file_load(path, cx);
                }
            });
            if let Err(error) = result {
                tracing::error!(error = %error, "failed to update Emerald while opening a file");
            }
        })
        .detach();
    }

    pub(super) fn delete_current_file(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let receiver = window.prompt(
            PromptLevel::Warning,
            "Delete the current file?",
            Some("This permanently removes the file from the workspace."),
            &["Delete", "Cancel"],
            cx,
        );
        cx.spawn(async move |view, cx| {
            match receiver.await {
                Ok(0) => {}
                Ok(_) => return,
                Err(error) => {
                    tracing::warn!(error = %error, "delete confirmation was unavailable");
                    return;
                }
            }
            let result = view.update(cx, |view, cx| {
                if let Err(error) = view.state.delete_active_file() {
                    view.show_operation_error(error, cx);
                    return;
                }
                view.interaction.file_load_error = None;
                view.source_list_state.scroll_to(ListOffset {
                    item_ix: 0,
                    offset_in_item: px(0.0),
                });
                view.interaction.source_revealed = true;
                view.schedule_parse_refresh(cx);
                view.blink_cursor.show();
                cx.notify();
            });
            if let Err(error) = result {
                tracing::error!(error = %error, "failed to update Emerald while deleting a file");
            }
        })
        .detach();
    }

    pub(super) fn begin_rename(
        &mut self,
        file: PathBuf,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.interaction.renaming {
            return;
        }
        if file != self.state.active_file() {
            if !self.file_loading() {
                self.start_file_load(file, cx);
            }
            return;
        }
        self.interaction.rename_buffer = self
            .state
            .active_file()
            .file_stem()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        self.interaction.renaming = true;
        self.interaction.context_menu = None;
        self.blink_cursor.show();
        window.focus(&self.focus_handle, cx);
        cx.notify();
    }
}
