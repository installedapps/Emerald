use crate::theme::EmeraldTheme;
use crate::Selection;

pub const SOURCE_LINE_HEIGHT: f32 = 28.0;
/// Source rendering measures this font's actual glyph positions for hit testing.
pub const SOURCE_FONT_FAMILY: &str = "monospace";
/// Legacy layout estimate; interactive source hit testing uses painted geometry.
pub const SOURCE_CHAR_WIDTH: f32 = 9.5;
pub const DOCUMENT_PADDING_X: f32 = 24.0;
pub const DOCUMENT_PADDING_Y: f32 = 20.0;

/// The x coordinate where source text begins in window coordinates. Mouse
/// events use window coordinates, while the source column is relative to the
/// text inside the document panel.
pub fn source_text_origin_x(viewport_width: f32) -> f32 {
    let layout = shell_layout(viewport_width);
    let editor_margin = editor_chrome(layout).margin;
    let sidebar_width = match layout.mode {
        LayoutMode::Desktop => layout.sidebar_width,
        LayoutMode::Compact => 0.0,
    };

    sidebar_width + editor_margin + 1.0 + DOCUMENT_PADDING_X
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum LayoutMode {
    Desktop,
    Compact,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ShellLayout {
    pub mode: LayoutMode,
    pub sidebar_width: f32,
    pub sidebar_height: Option<f32>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SidebarFileStyle {
    pub background: u32,
    pub text: u32,
    pub border: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SidebarFileListChrome {
    pub scrollable: bool,
    pub max_visible_files: usize,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DocumentScrollChrome {
    pub scrollable: bool,
    pub horizontal: bool,
    pub vertical: bool,
    pub min_lines_for_scroll: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DocumentSurfaceChrome {
    pub fill_available_height: bool,
    pub fill_available_width: bool,
    pub reserve_scrollbar_space: bool,
    pub allow_child_width_to_shrink: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScrollbarChrome {
    pub visible: bool,
    pub width: f32,
    pub hotzone_width: f32,
    pub color: u32,
    pub glow: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditorChrome {
    pub margin: f32,
    pub show_compact_header: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorStyle {
    pub color: u32,
    pub width: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CursorChrome {
    pub opacity: f32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ToolbarAction {
    pub id: &'static str,
    pub label: &'static str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NoteSwitchIntent {
    ExistingFile,
    NewFile,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceSegment {
    pub text: String,
    pub selected: bool,
    pub cursor_before: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceLine {
    pub start: usize,
    pub end: usize,
    pub segments: Vec<SourceSegment>,
    pub cursor_at_end: bool,
    pub newline_selected: bool,
}

pub fn shell_layout(viewport_width: f32) -> ShellLayout {
    if viewport_width < 760.0 {
        ShellLayout {
            mode: LayoutMode::Compact,
            sidebar_width: viewport_width,
            sidebar_height: Some(180.0),
        }
    } else {
        ShellLayout {
            mode: LayoutMode::Desktop,
            sidebar_width: 280.0,
            sidebar_height: None,
        }
    }
}

pub fn editor_chrome(layout: ShellLayout) -> EditorChrome {
    match layout.mode {
        LayoutMode::Compact => EditorChrome {
            margin: 12.0,
            show_compact_header: true,
        },
        LayoutMode::Desktop => EditorChrome {
            margin: 20.0,
            show_compact_header: false,
        },
    }
}

pub fn cursor_style(theme: EmeraldTheme) -> CursorStyle {
    CursorStyle {
        color: theme.accent,
        width: 2.0,
    }
}

pub fn cursor_chrome(visible: bool) -> CursorChrome {
    CursorChrome {
        opacity: if visible { 1.0 } else { 0.0 },
    }
}

pub fn document_line_for_drag(pointer_y: f32, scroll_y: f32, line_height: f32) -> usize {
    let line_height = line_height.max(1.0);
    let content_y = (pointer_y - scroll_y).max(0.0);

    (content_y / line_height).floor() as usize
}

pub fn document_column_for_pointer(
    pointer_x: f32,
    scroll_x: f32,
    padding_x: f32,
    char_width: f32,
) -> usize {
    let char_width = char_width.max(1.0);
    ((pointer_x - padding_x - scroll_x).max(0.0) / char_width).round() as usize
}

pub fn source_view_lines(
    text: &str,
    cursor: usize,
    selection: Option<Selection>,
) -> Vec<SourceLine> {
    wrapped_source_view_lines(text, cursor, selection, |_| Vec::new())
}

/// Soft wraps retain document byte offsets; only real newlines are selectable.
pub fn wrapped_source_view_lines(
    text: &str,
    cursor: usize,
    selection: Option<Selection>,
    mut wrap_boundaries: impl FnMut(&str) -> Vec<usize>,
) -> Vec<SourceLine> {
    let cursor = cursor.min(text.len());
    source_line_ranges(text)
        .into_iter()
        .flat_map(|range| {
            wrapped_source_line(
                &text[range.clone()],
                range.start,
                cursor,
                selection.as_ref(),
                &mut wrap_boundaries,
            )
        })
        .collect()
}

/// A lightweight index: no text copies, shaping, or selection segments.
pub fn source_line_ranges(text: &str) -> Vec<std::ops::Range<usize>> {
    let mut start = 0;
    let mut ranges = Vec::new();
    for line in text.split_inclusive('\n') {
        ranges.push(start..start + line.len());
        start += line.len();
    }
    if text.is_empty() || text.ends_with('\n') {
        ranges.push(start..start);
    }
    ranges
}

/// Shapes one logical line on demand, preserving absolute document offsets.
pub fn wrapped_source_line(
    line: &str,
    start: usize,
    cursor: usize,
    selection: Option<&Selection>,
    mut wrap_boundaries: impl FnMut(&str) -> Vec<usize>,
) -> Vec<SourceLine> {
    if line.is_empty() {
        return vec![source_line("", start, cursor, selection)];
    }
    let visible_line = line.strip_suffix('\n').unwrap_or(line);
    let mut row_start = 0;
    let mut rows = Vec::new();
    for row_end in wrap_boundaries(visible_line)
        .into_iter()
        .chain(std::iter::once(visible_line.len()))
    {
        let mut row = source_line(
            &visible_line[row_start..row_end],
            start + row_start,
            cursor,
            selection,
        );
        let last_row = row_end == visible_line.len();
        row.cursor_at_end &= last_row;
        row.newline_selected = last_row
            && line.ends_with('\n')
            && selection
                .is_some_and(|selection| selection.start <= row.end && row.end < selection.end);
        rows.push(row);
        row_start = row_end;
    }
    rows
}

fn source_line(
    line: &str,
    start: usize,
    cursor: usize,
    selection: Option<&Selection>,
) -> SourceLine {
    let end = start + line.len();
    let cursor_in_line = (start..=end).contains(&cursor);
    let mut segments = Vec::new();
    let mut current = String::new();
    let mut current_selected = None;

    for (byte_index, ch) in line.char_indices() {
        let ch_start = start + byte_index;
        let selected = selection
            .is_some_and(|selection| ch_start >= selection.start && ch_start < selection.end);
        let cursor_before = cursor_in_line && ch_start == cursor;

        if current_selected == Some(selected) && !cursor_before {
            current.push(ch);
            continue;
        }

        if !current.is_empty() {
            segments.push(SourceSegment {
                text: std::mem::take(&mut current),
                selected: current_selected.unwrap_or(false),
                cursor_before: false,
            });
        }

        current.push(ch);
        current_selected = Some(selected);
        if cursor_before {
            segments.push(SourceSegment {
                text: std::mem::take(&mut current),
                selected,
                cursor_before: true,
            });
            current_selected = None;
        }
    }

    if !current.is_empty() {
        segments.push(SourceSegment {
            text: current,
            selected: current_selected.unwrap_or(false),
            cursor_before: false,
        });
    }

    SourceLine {
        start,
        end,
        segments,
        cursor_at_end: cursor_in_line && cursor == end,
        newline_selected: false,
    }
}

pub fn toolbar_actions() -> [ToolbarAction; 4] {
    [
        ToolbarAction {
            id: "new-file",
            label: "New file",
        },
        ToolbarAction {
            id: "save",
            label: "Save",
        },
        ToolbarAction {
            id: "open",
            label: "Open",
        },
        ToolbarAction {
            id: "delete",
            label: "Delete",
        },
    ]
}

impl NoteSwitchIntent {
    pub fn reveals_source(self) -> bool {
        matches!(self, Self::NewFile)
    }
}

pub fn sidebar_file_style(theme: EmeraldTheme, active: bool, hovered: bool) -> SidebarFileStyle {
    if active {
        SidebarFileStyle {
            background: theme.active_file,
            text: theme.active_file_text,
            border: theme.accent,
        }
    } else if hovered {
        SidebarFileStyle {
            background: theme.hover,
            text: theme.text,
            border: theme.hover,
        }
    } else {
        SidebarFileStyle {
            background: theme.sidebar,
            text: theme.text,
            border: theme.sidebar,
        }
    }
}

pub fn sidebar_file_list_chrome(file_count: usize) -> SidebarFileListChrome {
    SidebarFileListChrome {
        scrollable: file_count > 18,
        max_visible_files: 18,
    }
}

pub fn document_scroll_chrome() -> DocumentScrollChrome {
    DocumentScrollChrome {
        scrollable: true,
        horizontal: false,
        vertical: true,
        min_lines_for_scroll: 25,
    }
}

pub fn document_surface_chrome() -> DocumentSurfaceChrome {
    DocumentSurfaceChrome {
        fill_available_height: true,
        fill_available_width: true,
        reserve_scrollbar_space: true,
        allow_child_width_to_shrink: true,
    }
}

pub fn scrollbar_chrome(theme: EmeraldTheme, scrollable: bool, hovered: bool) -> ScrollbarChrome {
    ScrollbarChrome {
        visible: scrollable,
        width: if hovered { 8.0 } else { 3.0 },
        hotzone_width: 18.0,
        color: theme.accent,
        glow: scrollable && hovered,
    }
}

#[cfg(test)]
#[path = "tests/ui.rs"]
mod tests;

#[cfg(test)]
#[path = "tests/ui_soft_wrap.rs"]
mod soft_wrap_tests;
