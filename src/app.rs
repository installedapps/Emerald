use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

#[allow(unused_imports)]
use emerald::cursor::BlinkCursor;
use emerald::rendered::{RenderBlock, RenderCache, RenderStyle};
use emerald::theme::{EmeraldTheme, ThemeId, ThemePreferences};
use emerald::ui::{
    cursor_style, document_scroll_chrome, document_surface_chrome, editor_chrome, shell_layout,
    sidebar_file_style, LayoutMode, NoteSwitchIntent, ShellLayout, DOCUMENT_PADDING_X,
    SOURCE_FONT_FAMILY, SOURCE_LINE_HEIGHT,
};
use emerald::{EditCommand, EditorState};
#[allow(unused_imports)]
use gpui::{
    div, list, prelude::*, px, rgb, size, uniform_list, AnyElement, App, Application, Bounds,
    ClipboardItem, Context, FocusHandle, Focusable, IntoElement, Keystroke, ListAlignment,
    ListOffset, ListState, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent,
    PathPromptOptions, PromptLevel, Render, Window, WindowBounds, WindowOptions,
};

pub(crate) struct Emerald {
    state: EditorState,
    interaction: InteractionState,
    render_cache: RenderCache,
    focus_handle: FocusHandle,
    source_list_state: ListState,
    document_list_state: ListState,
    document_list_count: usize,
    document_list_blocks: Option<Arc<Vec<RenderBlock>>>,
    document_list_file: Option<PathBuf>,
    pub(crate) blink_cursor: BlinkCursor,
    theme_preferences: ThemePreferences,
}

/// Ephemeral view state is isolated from the document/workspace session.
/// Keeping this state together makes interaction transitions explicit and
/// prevents feature modules from depending on unrelated document internals.
struct InteractionState {
    is_mouse_selecting: bool,
    editing_revision: u64,
    parse_refresh_revision: u64,
    source_revealed: bool,
    source_lines_cache: Option<SourceLinesCache>,
    source_wrap_width: f32,
    source_geometry: Rc<RefCell<Vec<source_geometry::SourceRowGeometry>>>,
    pending_source_click: Option<((f32, f32), bool)>,
    renaming: bool,
    rename_buffer: String,
    context_menu: Option<ContextMenu>,
    file_switch_revision: u64,
    file_loading: bool,
    file_load_error: Option<String>,
    graph_mode: bool,
    graph_zoom: f32,
    graph_offset: gpui::Point<f32>,
    graph_drag_origin: Option<gpui::Point<f32>>,
    graph_drag_start: gpui::Point<f32>,
    graph_hovered: Option<usize>,
    graph_settings: bool,
    favorites_only: bool,
    sidebar_visible: bool,
    sidebar_width: f32,
    sidebar_resize: Option<(f32, f32)>,
    links_sidebar_visible: bool,
    links_sidebar_width: f32,
    links_sidebar_resize: Option<(f32, f32)>,
}

#[derive(Clone)]
enum ContextMenu {
    Slash(emerald::slash_commands::SlashSearch),
    Editor {
        position: gpui::Point<gpui::Pixels>,
    },
    Sidebar {
        position: gpui::Point<gpui::Pixels>,
        file: PathBuf,
    },
    Themes {
        position: gpui::Point<gpui::Pixels>,
    },
}

struct SourceLinesCache {
    wrap_width: f32,
    theme_text: u32,
    text_revision: u64,
    file: PathBuf,
    text: Arc<str>,
    ranges: Arc<Vec<std::ops::Range<usize>>>,
    layouts: Rc<RefCell<source_layout::SourceLayoutCache>>,
}

const SOURCE_IDLE_PREVIEW_DELAY: Duration = Duration::from_millis(1800);
const PARSE_IDLE_DELAY: Duration = Duration::from_millis(650);

fn source_can_return_to_preview(dragging: bool, selection: Option<&emerald::Selection>) -> bool {
    !dragging && selection.is_none()
}

#[path = "context.rs"]
mod context;
#[path = "editor.rs"]
mod editor;
#[path = "graph_scene.rs"]
mod graph_scene;
#[path = "graph_view.rs"]
mod graph_view;
#[path = "input.rs"]
mod input;
#[path = "rendered_view.rs"]
mod rendered_view;
#[cfg(test)]
#[path = "tests/scrolling.rs"]
mod scrolling_tests;
#[path = "sidebar.rs"]
mod sidebar;
#[path = "sidebar_controls.rs"]
mod sidebar_controls;
#[path = "sidebar_frame.rs"]
mod sidebar_frame;
#[path = "sidebar_links.rs"]
mod sidebar_links;
#[path = "sidebar_resize.rs"]
mod sidebar_resize;
#[path = "slash_menu.rs"]
mod slash_menu;
#[path = "source.rs"]
mod source;
#[path = "source_geometry.rs"]
mod source_geometry;
#[path = "source_layout.rs"]
mod source_layout;
#[path = "workspace_actions.rs"]
mod workspace_actions;

impl Emerald {
    pub(crate) fn new(cx: &mut Context<Self>) -> Self {
        let workspace = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join("notes");
        tracing::info!(path = %workspace.display(), "opening workspace");
        let state = EditorState::loading_placeholder(&workspace).unwrap_or_else(|error| {
            tracing::error!(path = %workspace.display(), error = %error, "failed to open workspace");
            panic!("could not open the Emerald workspace at {}: {error}", workspace.display());
        });
        let mut view = Self::with_state(state, cx);
        view.start_initial_workspace_load(workspace, cx);
        view
    }

    fn with_state(state: EditorState, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();

        Self {
            state,
            theme_preferences: ThemePreferences::load(),
            interaction: InteractionState {
                is_mouse_selecting: false,
                editing_revision: 0,
                parse_refresh_revision: 0,
                source_revealed: false,
                source_lines_cache: None,
                source_wrap_width: 0.0,
                source_geometry: Rc::default(),
                pending_source_click: None,
                renaming: false,
                rename_buffer: String::new(),
                context_menu: None,
                file_switch_revision: 0,
                file_loading: false,
                file_load_error: None,
                graph_mode: false,
                graph_zoom: 1.0,
                graph_offset: gpui::point(0.0, 0.0),
                graph_drag_origin: None,
                graph_drag_start: gpui::point(0.0, 0.0),
                graph_hovered: None,
                graph_settings: false,
                favorites_only: false,
                sidebar_visible: true,
                sidebar_width: 280.0,
                sidebar_resize: None,
                links_sidebar_visible: true,
                links_sidebar_width: 260.0,
                links_sidebar_resize: None,
            },
            render_cache: RenderCache::default(),
            focus_handle,
            source_list_state: ListState::new(0, ListAlignment::Top, px(300.0)),
            document_list_state: ListState::new(0, ListAlignment::Top, px(300.0)),
            document_list_count: 0,
            document_list_blocks: None,
            document_list_file: None,
            blink_cursor: BlinkCursor::default(),
        }
    }

    pub(crate) fn theme(&self) -> EmeraldTheme {
        self.theme_preferences.selected.theme()
    }

    fn start_initial_workspace_load(&mut self, workspace: PathBuf, cx: &mut Context<Self>) {
        self.interaction.file_switch_revision =
            self.interaction.file_switch_revision.wrapping_add(1);
        let revision = self.interaction.file_switch_revision;
        let started = std::time::Instant::now();
        self.interaction.file_loading = true;

        cx.spawn(async move |view, cx| {
            let result = cx
                .background_executor()
                .spawn(async move {
                    let state = EditorState::open_or_create(&workspace)?;
                    let active_file = state.active_file().to_path_buf();
                    let snapshot = emerald::rendered::RenderSnapshot::from_parsed(
                        state.text(),
                        state.parsed_document().clone(),
                    );
                    Ok::<_, anyhow::Error>((state, active_file, snapshot))
                })
                .await;

            let _ = view.update(cx, |view, cx| {
                if view.interaction.file_switch_revision != revision {
                    return;
                }

                view.interaction.file_loading = false;
                match result {
                    Ok((state, active_file, snapshot)) => {
                        view.state = state;
                        view.render_cache = RenderCache::default();
                        view.render_cache
                            .insert(active_file, view.state.text(), snapshot);
                        view.source_list_state.reset(0);
                        view.document_list_state.reset(0);
                        view.document_list_count = 0;
                        view.document_list_blocks = None;
                        view.document_list_file = None;
                        view.interaction.source_lines_cache = None;
                        view.interaction.file_load_error = None;
                        tracing::info!(
                            elapsed_ms = started.elapsed().as_millis(),
                            files = view.state.files().len(),
                            lines = view.state.line_count(),
                            "workspace loaded"
                        );
                    }
                    Err(error) => {
                        tracing::error!(error = %error, "failed to open the Emerald workspace");
                        view.interaction.file_load_error = Some(error.to_string());
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    pub(crate) fn select_theme(&mut self, theme: ThemeId, cx: &mut Context<Self>) {
        self.theme_preferences.selected = theme;
        self.theme_preferences.save();
        self.interaction.context_menu = None;
        cx.notify();
    }

    pub(crate) fn toggle_theme_favorite(&mut self, theme: ThemeId, cx: &mut Context<Self>) {
        if self.theme_preferences.favorites.contains(&theme) {
            self.theme_preferences
                .favorites
                .retain(|favorite| *favorite != theme);
        } else {
            self.theme_preferences.favorites.push(theme);
        }
        self.theme_preferences.save();
        cx.notify();
    }

    fn reveal_source(&mut self, cx: &mut Context<Self>) {
        self.interaction.source_revealed = true;
        self.interaction.editing_revision = self.interaction.editing_revision.wrapping_add(1);
        let revision = self.interaction.editing_revision;

        cx.spawn(async move |view, cx| {
            cx.background_executor()
                .timer(SOURCE_IDLE_PREVIEW_DELAY)
                .await;
            let _ = view.update(cx, |view: &mut Emerald, cx| {
                if !matches!(view.interaction.context_menu, Some(ContextMenu::Slash(_)))
                    && view.interaction.editing_revision == revision
                    && source_can_return_to_preview(
                        view.interaction.is_mouse_selecting,
                        view.state.selection().as_ref(),
                    )
                {
                    view.interaction.source_revealed = false;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn schedule_parse_refresh(&mut self, cx: &mut Context<Self>) {
        self.interaction.parse_refresh_revision =
            self.interaction.parse_refresh_revision.wrapping_add(1);
        let revision = self.interaction.parse_refresh_revision;

        cx.spawn(async move |view, cx| {
            cx.background_executor().timer(PARSE_IDLE_DELAY).await;
            let Ok(Some((file, text_revision, text))) = view.update(cx, |view: &mut Emerald, _| {
                (view.interaction.parse_refresh_revision == revision && view.state.parse_is_dirty())
                    .then(|| {
                        (
                            view.state.active_file().to_path_buf(),
                            view.state.text_revision(),
                            view.state.text().to_owned(),
                        )
                    })
            }) else {
                return;
            };
            let parsed = cx
                .background_executor()
                .spawn(async move { emerald::parser::ParsedDocument::from_source(&text) })
                .await;
            let _ = view.update(cx, |view: &mut Emerald, cx| {
                if view.state.active_file() == file
                    && view.state.apply_parse_result(text_revision, parsed)
                {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    pub(crate) fn file_loading(&self) -> bool {
        self.interaction.file_loading
    }
}

impl Focusable for Emerald {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Render for Emerald {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let layout = shell_layout(f32::from(window.viewport_size().width));
        let shell = div()
            .flex()
            .size_full()
            .min_w(px(0.0))
            .bg(rgb(self.theme().window_background));
        let shell = match layout.mode {
            LayoutMode::Compact => shell.flex_col(),
            LayoutMode::Desktop => shell.flex_row(),
        };
        let shell = shell
            .child(self.render_sidebar(layout, cx))
            .child(self.render_editor(layout, window, cx))
            .child(self.render_links_sidebar(cx))
            .on_mouse_move(cx.listener(Self::resize_sidebar))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(Self::finish_sidebar_resize),
            )
            .on_mouse_down(MouseButton::Left, cx.listener(Self::dismiss_context_menu));
        if let Some(menu) = self.interaction.context_menu.clone() {
            shell.child(self.render_context_menu(menu, cx))
        } else {
            shell
        }
    }
}

#[cfg(test)]
#[path = "tests/sidebar_interactions.rs"]
mod sidebar_interaction_tests;
#[cfg(test)]
#[path = "tests/sidebar_unit.rs"]
mod sidebar_unit_tests;
#[cfg(test)]
#[path = "tests/app.rs"]
mod tests;
