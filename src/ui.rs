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

pub fn sidebar_file_style(theme: EmeraldTheme, active: bool) -> SidebarFileStyle {
    if active {
        SidebarFileStyle {
            background: theme.active_file,
            text: theme.active_file_text,
            border: theme.accent,
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
mod tests {
    use super::*;
    use crate::theme::EVERFOREST_DARK;

    #[test]
    fn lazy_source_rows_match_full_document_selection_and_offsets() {
        let text = "offscreen\na界b c\n\nlast\n";
        let selection = Some(Selection { start: 11, end: 19 });
        let expected = wrapped_source_view_lines(text, 14, selection.clone(), |line| {
            if line == "a界b c" {
                vec![4]
            } else {
                vec![]
            }
        });
        let ranges = source_line_ranges(text);
        let mut shaped = Vec::new();
        let visible = wrapped_source_line(
            &text[ranges[1].clone()],
            ranges[1].start,
            14,
            selection.as_ref(),
            |line| {
                shaped.push(line.to_owned());
                vec![4]
            },
        );
        assert_eq!(shaped, vec!["a界b c"]);
        assert_eq!(visible, expected[1..3]);
        assert_eq!(ranges.last(), Some(&(text.len()..text.len())));
    }

    #[test]
    fn lazy_source_does_not_clamp_offscreen_cursor_to_visible_line() {
        let rows = wrapped_source_line("visible\n", 20, 100, None, |_| vec![]);
        assert!(!rows[0].cursor_at_end);
        assert!(rows[0]
            .segments
            .iter()
            .all(|segment| !segment.cursor_before));
        assert_eq!(source_line_ranges(""), vec![0..0]);
        assert_eq!(source_line_ranges("a\n\nb"), vec![0..2, 2..3, 3..4]);
    }

    #[test]
    fn compact_layout_stacks_sidebar_above_editor() {
        let layout = shell_layout(520.0);

        assert_eq!(layout.mode, LayoutMode::Compact);
        assert_eq!(layout.sidebar_width, 520.0);
        assert_eq!(layout.sidebar_height, Some(180.0));
    }

    #[test]
    fn desktop_layout_keeps_obsidian_style_side_rail() {
        let layout = shell_layout(1200.0);

        assert_eq!(layout.mode, LayoutMode::Desktop);
        assert_eq!(layout.sidebar_width, 280.0);
        assert_eq!(layout.sidebar_height, None);
    }

    #[test]
    fn active_sidebar_file_uses_clear_complementary_highlight() {
        let active = sidebar_file_style(EVERFOREST_DARK, true);
        let inactive = sidebar_file_style(EVERFOREST_DARK, false);

        assert_ne!(active.background, inactive.background);
        assert_eq!(active.background, EVERFOREST_DARK.active_file);
        assert_eq!(active.border, EVERFOREST_DARK.accent);
    }

    #[test]
    fn editor_chrome_uses_compact_spacing_on_small_viewports() {
        let chrome = editor_chrome(shell_layout(520.0));

        assert_eq!(chrome.margin, 12.0);
        assert!(chrome.show_compact_header);
    }

    #[test]
    fn editor_chrome_uses_more_breathing_room_on_desktop() {
        let chrome = editor_chrome(shell_layout(1200.0));

        assert_eq!(chrome.margin, 20.0);
        assert!(!chrome.show_compact_header);
    }

    #[test]
    fn cursor_style_is_visible_against_the_editor_panel() {
        let cursor = cursor_style(EVERFOREST_DARK);

        assert_eq!(cursor.color, EVERFOREST_DARK.accent);
        assert!(cursor.width >= 2.0);
    }

    #[test]
    fn cursor_chrome_hides_and_shows_the_blinking_caret() {
        assert_eq!(cursor_chrome(true).opacity, 1.0);
        assert_eq!(cursor_chrome(false).opacity, 0.0);
    }

    #[test]
    fn document_drag_line_accounts_for_scroll_offset() {
        assert_eq!(document_line_for_drag(52.0, 0.0, 26.0), 2);
        assert_eq!(document_line_for_drag(52.0, -52.0, 26.0), 4);
    }

    #[test]
    fn document_drag_line_changes_only_at_row_boundaries() {
        assert_eq!(document_line_for_drag(0.0, 0.0, 28.0), 0);
        assert_eq!(document_line_for_drag(27.9, 0.0, 28.0), 0);
        assert_eq!(document_line_for_drag(28.0, 0.0, 28.0), 1);
    }

    #[test]
    fn document_column_for_pointer_accounts_for_padding_and_horizontal_scroll() {
        assert_eq!(document_column_for_pointer(24.0, 0.0, 24.0, 10.0), 0);
        assert_eq!(document_column_for_pointer(54.0, 0.0, 24.0, 10.0), 3);
        assert_eq!(document_column_for_pointer(54.0, -20.0, 24.0, 10.0), 5);
    }

    #[test]
    fn document_column_for_pointer_never_becomes_negative_and_rounds_to_nearest_char() {
        assert_eq!(document_column_for_pointer(0.0, 0.0, 24.0, 10.0), 0);
        assert_eq!(document_column_for_pointer(38.9, 0.0, 24.0, 10.0), 1);
        assert_eq!(document_column_for_pointer(39.1, 0.0, 24.0, 10.0), 2);
    }

    #[test]
    fn source_text_origin_accounts_for_desktop_sidebar_and_editor_spacing() {
        assert_eq!(source_text_origin_x(1200.0), 325.0);
    }

    #[test]
    fn source_text_origin_accounts_for_compact_editor_spacing() {
        assert_eq!(source_text_origin_x(520.0), 37.0);
    }

    #[test]
    fn source_view_keeps_a_trailing_empty_line() {
        let lines = source_view_lines("alpha\n", 6, None);

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0].start, 0);
        assert_eq!(lines[0].end, 5);
        assert_eq!(lines[1].start, 6);
        assert_eq!(lines[1].end, 6);
        assert!(lines[1].cursor_at_end);
    }

    #[test]
    fn source_view_places_cursor_at_newline_boundary_on_previous_line() {
        let lines = source_view_lines("a\nb", 1, None);

        assert_eq!(lines.len(), 2);
        assert!(lines[0].cursor_at_end);
        assert!(!lines[1].cursor_at_end);
        assert!(lines[1]
            .segments
            .iter()
            .all(|segment| !segment.cursor_before));
    }

    #[test]
    fn source_view_places_cursor_at_start_of_next_line_after_newline() {
        let lines = source_view_lines("a\nb", 2, None);

        assert!(!lines[0].cursor_at_end);
        assert!(lines[1].segments[0].cursor_before);
        assert_eq!(lines[1].segments[0].text, "b");
    }

    #[test]
    fn source_view_splits_only_around_selection_and_cursor() {
        let lines = source_view_lines("abcdef", 3, Some(Selection { start: 1, end: 5 }));

        assert_eq!(
            lines[0].segments,
            vec![
                SourceSegment {
                    text: "a".to_string(),
                    selected: false,
                    cursor_before: false,
                },
                SourceSegment {
                    text: "bc".to_string(),
                    selected: true,
                    cursor_before: false,
                },
                SourceSegment {
                    text: "d".to_string(),
                    selected: true,
                    cursor_before: true,
                },
                SourceSegment {
                    text: "e".to_string(),
                    selected: true,
                    cursor_before: false,
                },
                SourceSegment {
                    text: "f".to_string(),
                    selected: false,
                    cursor_before: false,
                },
            ]
        );
    }

    #[test]
    fn source_view_selection_can_span_lines() {
        let lines = source_view_lines(
            "one\ntwo",
            "one\ntwo".len(),
            Some(Selection { start: 2, end: 6 }),
        );

        assert_eq!(lines[0].segments[0].text, "on");
        assert!(!lines[0].segments[0].selected);
        assert_eq!(lines[0].segments[1].text, "e");
        assert!(lines[0].segments[1].selected);
        assert_eq!(lines[1].segments[0].text, "tw");
        assert!(lines[1].segments[0].selected);
        assert_eq!(lines[1].segments[1].text, "o");
        assert!(!lines[1].segments[1].selected);
    }

    #[test]
    fn selection_marks_newlines_including_otherwise_empty_rows() {
        let lines = source_view_lines("a\n\nb", 3, Some(Selection { start: 1, end: 3 }));
        assert!(lines[0].newline_selected);
        assert!(lines[1].newline_selected);
        assert!(!lines[2].newline_selected);
    }

    #[test]
    fn repeated_text_is_highlighted_only_at_its_selected_byte_range() {
        let lines = source_view_lines("same same\nsame", 9, Some(Selection { start: 5, end: 9 }));
        let selected: Vec<_> = lines
            .iter()
            .map(|line| {
                line.segments
                    .iter()
                    .filter(|segment| segment.selected)
                    .map(|segment| segment.text.as_str())
                    .collect::<String>()
            })
            .collect();
        assert_eq!(selected, vec!["same", ""]);
        assert!(!lines[0].segments[0].selected);
    }

    #[test]
    fn toolbar_actions_expose_new_file_and_save() {
        let actions = toolbar_actions();

        assert!(actions.iter().any(|action| action.id == "new-file"));
        assert!(actions.iter().any(|action| action.id == "save"));
        assert!(actions.iter().any(|action| action.id == "open"));
        assert!(actions.iter().any(|action| action.id == "delete"));
    }

    #[test]
    fn switching_existing_notes_resets_scroll_without_forcing_source_mode() {
        assert!(!NoteSwitchIntent::ExistingFile.reveals_source());
    }

    #[test]
    fn creating_a_note_resets_scroll_and_opens_source_mode() {
        assert!(NoteSwitchIntent::NewFile.reveals_source());
    }

    #[test]
    fn sidebar_file_list_becomes_scrollable_for_long_file_sets() {
        let chrome = sidebar_file_list_chrome(42);

        assert!(chrome.scrollable);
        assert_eq!(chrome.max_visible_files, 18);
    }

    #[test]
    fn document_scroll_chrome_enables_scroll_for_long_documents() {
        let chrome = document_scroll_chrome();

        assert!(chrome.scrollable);
        assert!(chrome.vertical);
        assert!(!chrome.horizontal);
        assert_eq!(chrome.min_lines_for_scroll, 25);
    }

    #[test]
    fn document_scroll_chrome_keeps_vertical_canvas_available_for_short_documents() {
        let chrome = document_scroll_chrome();

        assert!(chrome.scrollable);
        assert!(chrome.vertical);
        assert!(!chrome.horizontal);
    }

    #[test]
    fn document_surface_fills_available_space_even_for_short_files() {
        let chrome = document_surface_chrome();

        assert!(chrome.fill_available_height);
        assert!(chrome.fill_available_width);
        assert!(chrome.reserve_scrollbar_space);
        assert!(chrome.allow_child_width_to_shrink);
    }

    #[test]
    fn document_surface_does_not_request_full_width_plus_scrollbar_hotzone() {
        let chrome = document_surface_chrome();

        assert!(chrome.allow_child_width_to_shrink);
        assert!(chrome.reserve_scrollbar_space);
    }

    #[test]
    fn scrollbar_appears_and_grows_with_glow_on_hover() {
        let hidden = scrollbar_chrome(EVERFOREST_DARK, true, false);
        let hovered = scrollbar_chrome(EVERFOREST_DARK, true, true);

        assert!(hidden.visible);
        assert!(hovered.visible);
        assert!(hovered.width > hidden.width);
        assert!(hovered.glow);
        assert_eq!(hovered.color, EVERFOREST_DARK.accent);
    }
}

#[cfg(test)]
mod soft_wrap_tests {
    use super::*;

    #[test]
    fn soft_wraps_preserve_unicode_selection_and_own_the_caret_once() {
        let rows =
            wrapped_source_view_lines("a界b c\n", 4, Some(Selection { start: 1, end: 8 }), |_| {
                vec![4]
            });
        assert_eq!(
            rows.iter()
                .map(|row| (row.start, row.end))
                .collect::<Vec<_>>(),
            vec![(0, 4), (4, 7), (8, 8)]
        );
        assert!(!rows[0].cursor_at_end);
        assert!(rows[1].segments[0].cursor_before);
        assert!(!rows[0].newline_selected);
        assert!(rows[1].newline_selected);
        let selected: String = rows
            .iter()
            .flat_map(|row| &row.segments)
            .filter(|segment| segment.selected)
            .map(|segment| segment.text.as_str())
            .collect();
        assert_eq!(selected, "界b c");
        assert!(wrapped_source_view_lines("", 0, None, |_| vec![])[0].cursor_at_end);
    }
}
