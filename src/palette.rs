use super::*;
use gpui::{Animation, AnimationExt};

#[derive(Clone)]
pub(super) struct CommandPalette {
    pub query: String,
    pub selected: usize,
    pub mode: PaletteMode,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum PaletteMode {
    Commands,
    Themes,
}

#[derive(Clone, Copy)]
enum PaletteCommand {
    Fold,
    Unfold,
    NewNote,
    Theme,
    AddTag,
}

const COMMANDS: [(PaletteCommand, &str, &str); 5] = [
    (
        PaletteCommand::Fold,
        "Fold headings",
        "Collapse section content in preview",
    ),
    (
        PaletteCommand::Unfold,
        "Unfold headings",
        "Show all section content",
    ),
    (
        PaletteCommand::NewNote,
        "Create new note",
        "Start a note in this workspace",
    ),
    (
        PaletteCommand::Theme,
        "Change theme",
        "Choose an Emerald color theme",
    ),
    (
        PaletteCommand::AddTag,
        "Add tag…",
        "Add a tag to this note's AsciiDoc header",
    ),
];

impl CommandPalette {
    fn commands(&self) -> Vec<(PaletteCommand, &'static str, &'static str)> {
        let query = self.query.trim().to_lowercase();
        COMMANDS
            .iter()
            .copied()
            .filter(|(_, label, _)| {
                query.is_empty()
                    || label.to_lowercase().contains(&query)
                    || (query.starts_with("new note") && matches!(label, &"Create new note"))
                    || (query.starts_with("add tag") && matches!(label, &"Add tag…"))
                    || (query.starts_with("tag ") && matches!(label, &"Add tag…"))
                    || (query == "tag" && matches!(label, &"Add tag…"))
                    || (query == "fold" && matches!(label, &"Fold headings"))
                    || (query == "unfold" && matches!(label, &"Unfold headings"))
                    || (query == "theme" && matches!(label, &"Change theme"))
            })
            .collect()
    }

    pub(super) fn command_count(&self) -> usize {
        self.commands().len()
    }
}

impl Emerald {
    pub(super) fn render_command_palette(
        &self,
        palette: CommandPalette,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let theme = self.theme();
        let mut rows = Vec::new();
        match palette.mode {
            PaletteMode::Commands => {
                let commands = palette.commands();
                if commands.is_empty() {
                    rows.push(
                        div()
                            .px_2()
                            .py_3()
                            .text_color(rgb(theme.faint_text))
                            .child("No matching commands")
                            .into_any_element(),
                    );
                }
                rows.extend(
                    commands
                        .iter()
                        .enumerate()
                        .map(|(index, (_, label, help))| {
                            div()
                                .id(gpui::SharedString::from(format!("palette-command-{index}")))
                                .w_full()
                                .flex()
                                .items_center()
                                .justify_between()
                                .gap_4()
                                .px_2()
                                .py_2()
                                .rounded_sm()
                                .text_color(rgb(theme.text))
                                .when(index == palette.selected, |row| {
                                    row.bg(rgb(theme.selection))
                                })
                                .hover(|row| row.bg(rgb(theme.hover)))
                                .cursor_pointer()
                                .child(*label)
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(rgb(theme.faint_text))
                                        .child(*help),
                                )
                                .on_mouse_down(
                                    MouseButton::Left,
                                    cx.listener(move |view, _, window, cx| {
                                        view.run_palette_command(index, window, cx);
                                    }),
                                )
                                .into_any_element()
                        }),
                );
            }
            PaletteMode::Themes => {
                let shown = ThemeId::ALL
                    .into_iter()
                    .filter(|id| {
                        palette.query.is_empty()
                            || id
                                .name()
                                .to_lowercase()
                                .contains(&palette.query.to_lowercase())
                    })
                    .collect::<Vec<_>>();
                rows.extend(shown.into_iter().enumerate().map(|(index, id)| {
                    let active = self.theme_preferences.selected == id;
                    div()
                        .id(gpui::SharedString::from(format!(
                            "palette-theme-{}",
                            id.id()
                        )))
                        .w_full()
                        .flex()
                        .items_center()
                        .justify_between()
                        .px_3()
                        .py_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .text_color(rgb(theme.text))
                        .when(index == palette.selected || active, |row| {
                            row.bg(rgb(if active { theme.selection } else { theme.hover }))
                        })
                        .hover(|row| row.bg(rgb(theme.hover)))
                        .child(
                            div()
                                .flex()
                                .items_center()
                                .gap_2()
                                .child(
                                    div()
                                        .size(px(9.0))
                                        .rounded_full()
                                        .bg(rgb(id.theme().accent)),
                                )
                                .child(id.name()),
                        )
                        .when(active, |row| {
                            row.child(
                                div()
                                    .px_1()
                                    .rounded_sm()
                                    .bg(rgb(theme.accent))
                                    .text_xs()
                                    .text_color(rgb(theme.window_background))
                                    .child("ACTIVE"),
                            )
                        })
                        .on_mouse_down(
                            MouseButton::Left,
                            cx.listener(move |view, _, _, cx| view.choose_palette_theme(id, cx)),
                        )
                        .into_any_element()
                }));
            }
        }

        let closing = self.interaction.palette_closing;
        let revision = self.interaction.palette_revision;
        let panel = div()
            .id("command-palette")
            .w_full()
            .max_w(px(680.0))
            .p_2()
            .rounded_lg()
            .border_1()
            .border_color(rgb(if palette.mode == PaletteMode::Commands {
                theme.accent
            } else {
                theme.panel_border
            }))
            .bg(rgb(theme.menu))
            .text_color(rgb(theme.text))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::keep_palette_open))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .w_full()
                    .px_3()
                    .py_2()
                    .border_1()
                    .rounded_sm()
                    .border_color(rgb(if palette.mode == PaletteMode::Commands {
                        theme.accent
                    } else {
                        theme.panel_border
                    }))
                    .text_color(rgb(theme.text))
                    .child(div().text_color(rgb(theme.accent)).child(":"))
                    .child(format!("{}▏", palette.query)),
            )
            .child(div().mt_2().children(rows))
            .child(
                div()
                    .mt_2()
                    .px_2()
                    .text_xs()
                    .text_color(rgb(theme.faint_text))
                    .child(if palette.mode == PaletteMode::Themes {
                        "↑↓ to navigate    ↵ to use    esc to dismiss"
                    } else {
                        "↑↓ to navigate    ↵ to run    esc to dismiss"
                    }),
            )
            .with_animation(
                gpui::SharedString::from(format!("palette-panel-{revision}-{closing}")),
                Animation::new(Duration::from_millis(140)),
                move |element, phase| {
                    if closing {
                        element.opacity(1.0 - phase).mt(px(6.0 * phase))
                    } else {
                        element.opacity(phase).mt(px(6.0 * (1.0 - phase)))
                    }
                },
            );
        let overlay = div()
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .bg(rgb(0x000000).opacity(0.48))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::close_palette))
            .child(panel);
        let overlay = if closing {
            overlay.with_animation(
                gpui::SharedString::from(format!("palette-overlay-out-{revision}")),
                Animation::new(Duration::from_millis(140)),
                |element, phase| element.opacity(1.0 - phase),
            )
        } else {
            overlay.with_animation(
                gpui::SharedString::from(format!("palette-overlay-in-{revision}")),
                Animation::new(Duration::from_millis(140)),
                |element, phase| element.opacity(phase),
            )
        };
        overlay.into_any_element()
    }

    pub(super) fn run_palette_command(
        &mut self,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let commands = self
            .interaction
            .palette
            .as_ref()
            .map(CommandPalette::commands)
            .unwrap_or_default();
        let Some((command, _, _)) = commands.get(index).copied() else {
            return;
        };
        let query = self
            .interaction
            .palette
            .as_ref()
            .map(|palette| palette.query.trim().to_owned())
            .unwrap_or_default();
        match command {
            PaletteCommand::Fold => {
                self.interaction.headings_folded = true;
                self.interaction.editor_view_mode = PaletteEditorViewMode::Preview;
                self.interaction.source_revealed = false;
                self.dismiss_palette(cx);
            }
            PaletteCommand::Unfold => {
                self.interaction.headings_folded = false;
                self.interaction.editor_view_mode = PaletteEditorViewMode::Preview;
                self.interaction.source_revealed = false;
                self.dismiss_palette(cx);
            }
            PaletteCommand::NewNote => {
                let name = query
                    .strip_prefix("new note")
                    .map(str::trim)
                    .filter(|name| !name.is_empty())
                    .unwrap_or("untitled");
                if self.file_loading() {
                    self.dismiss_palette(cx);
                    return;
                }
                if let Err(error) = self.state.create_file(name) {
                    self.show_operation_error(error, cx);
                } else {
                    self.interaction.graph_mode = false;
                    self.interaction.source_revealed = true;
                    self.interaction.editor_view_mode = PaletteEditorViewMode::Source;
                    self.interaction.headings_folded = false;
                    self.interaction.file_load_error = None;
                    self.note_changed("New note created", cx);
                }
                self.dismiss_palette(cx);
                window.focus(&self.focus_handle, cx);
            }
            PaletteCommand::Theme => {
                if let Some(palette) = self.interaction.palette.as_mut() {
                    palette.mode = PaletteMode::Themes;
                    palette.query.clear();
                    palette.selected = ThemeId::ALL
                        .iter()
                        .position(|id| *id == self.theme_preferences.selected)
                        .unwrap_or(0);
                }
            }
            PaletteCommand::AddTag => {
                let tag = query
                    .strip_prefix("add tag")
                    .or_else(|| query.strip_prefix("tag"))
                    .map(str::trim)
                    .unwrap_or("");
                if !tag.is_empty() {
                    self.add_document_tag(tag, cx);
                }
                self.dismiss_palette(cx);
            }
        }
        if self.interaction.palette.is_none() {
            window.focus(&self.focus_handle, cx);
        }
        cx.notify();
    }

    pub(super) fn choose_palette_theme(&mut self, id: ThemeId, cx: &mut Context<Self>) {
        self.theme_preferences.selected = id;
        self.theme_preferences.save();
        if let Some(palette) = self.interaction.palette.as_mut() {
            palette.selected = ThemeId::ALL
                .iter()
                .position(|candidate| *candidate == id)
                .unwrap_or(0);
        }
        cx.notify();
    }

    fn add_document_tag(&mut self, tag: &str, cx: &mut Context<Self>) {
        let text = self.state.text().to_owned();
        let mut insert_at = 0;
        let mut prefix_newline = false;
        if text.starts_with("= ") {
            if let Some(end) = text.find('\n') {
                insert_at = end + 1;
            } else {
                insert_at = text.len();
                prefix_newline = true;
            }
        }
        let header_end = asciidoc_header_end(&text);
        if let Some(line_start) = text[..header_end].rfind(":tags:") {
            let line_end = text[line_start..]
                .find('\n')
                .map(|offset| line_start + offset)
                .unwrap_or(header_end);
            let existing = text[line_start + 6..line_end].trim();
            if existing.split(',').any(|existing| existing.trim() == tag) {
                return;
            }
            self.state.set_cursor(line_end, false);
            self.state.insert_text(&if existing.is_empty() {
                format!(" {tag}")
            } else {
                format!(", {tag}")
            });
        } else {
            self.state.set_cursor(insert_at, false);
            let separator = if prefix_newline { "\n" } else { "" };
            let trailing_separator = if text.starts_with("= ") { "" } else { "\n" };
            self.state
                .insert_text(&format!("{separator}:tags: {tag}\n{trailing_separator}"));
        }
        self.schedule_parse_refresh(cx);
        self.reveal_source(cx);
    }

    pub(super) fn close_palette(
        &mut self,
        _: &MouseDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.dismiss_palette(cx);
        window.focus(&self.focus_handle, cx);
    }

    pub(super) fn dismiss_palette(&mut self, cx: &mut Context<Self>) {
        if self.interaction.palette.is_none() || self.interaction.palette_closing {
            return;
        }
        self.interaction.palette_closing = true;
        self.interaction.palette_revision = self.interaction.palette_revision.wrapping_add(1);
        let revision = self.interaction.palette_revision;
        cx.notify();
        cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(150))
                .await;
            let _ = view.update(cx, |view, cx| {
                if view.interaction.palette_revision == revision && view.interaction.palette_closing
                {
                    view.interaction.palette = None;
                    view.interaction.palette_closing = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn keep_palette_open(&mut self, _: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        cx.stop_propagation();
    }
}

fn asciidoc_header_end(source: &str) -> usize {
    let mut offset = 0;
    for line in source.split_inclusive('\n') {
        if line.trim().is_empty() {
            return offset;
        }
        offset += line.len();
    }
    source.len()
}

// Keep this assignment local to the palette so the command action does not
// expose editor internals in the UI model.
use super::EditorViewMode as PaletteEditorViewMode;
