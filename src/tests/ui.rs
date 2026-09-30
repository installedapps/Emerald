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
    let active = sidebar_file_style(EVERFOREST_DARK, true, false);
    let inactive = sidebar_file_style(EVERFOREST_DARK, false, false);

    assert_ne!(active.background, inactive.background);
    assert_eq!(active.background, EVERFOREST_DARK.active_file);
    assert_eq!(active.border, EVERFOREST_DARK.accent);
}

#[test]
fn hovering_inactive_sidebar_file_uses_a_quiet_hover_state() {
    let hovered = sidebar_file_style(EVERFOREST_DARK, false, true);
    let active = sidebar_file_style(EVERFOREST_DARK, true, false);

    assert_ne!(hovered, active);
    assert_eq!(hovered.background, EVERFOREST_DARK.hover);
    assert_eq!(hovered.text, EVERFOREST_DARK.text);
}

#[test]
fn inactive_sidebar_file_returns_to_normal_colors_after_hover() {
    let before = sidebar_file_style(EVERFOREST_DARK, false, false);
    let during = sidebar_file_style(EVERFOREST_DARK, false, true);
    let after = sidebar_file_style(EVERFOREST_DARK, false, false);

    assert_ne!(during, before);
    assert_eq!(after, before);
}

#[test]
fn active_sidebar_file_stays_highlighted_after_hover_ends() {
    let active = sidebar_file_style(EVERFOREST_DARK, true, false);
    let active_hovered = sidebar_file_style(EVERFOREST_DARK, true, true);
    let inactive = sidebar_file_style(EVERFOREST_DARK, false, false);

    assert_eq!(active_hovered, active);
    assert_ne!(inactive, active);
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
