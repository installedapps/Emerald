use super::*;

#[test]
fn background_parse_rejects_stale_results_and_accepts_current_content() {
    let mut document = DocumentEditor::from_text("= Title");
    document.insert_text(" one");
    let revision = document.text_revision();
    let stale = ParsedDocument::from_source(document.text());
    document.insert_text(" two");
    assert!(!document.apply_parse_result(revision, stale));
    assert!(document.parse_is_dirty());
    let current = ParsedDocument::from_source(document.text());
    let revision = document.text_revision();
    assert!(document.apply_parse_result(revision, current.clone()));
    assert!(!document.parse_is_dirty());
    assert_eq!(document.parsed_document(), &current);
    assert!(!document.apply_parse_result(revision, current));
}

#[test]
fn left_and_right_collapse_a_selection_to_its_corresponding_edge() {
    for (anchor, cursor) in [(1, 5), (5, 1)] {
        for (command, expected) in [
            (EditCommand::MoveLeft { selecting: false }, 1),
            (EditCommand::MoveRight { selecting: false }, 5),
        ] {
            let mut document = DocumentEditor::from_text("abcdef");
            document.begin_mouse_selection(anchor);
            document.update_mouse_selection(cursor);
            document.end_mouse_selection();
            document.apply_edit_command(command);
            assert_eq!(document.cursor(), expected);
            assert_eq!(document.selection(), None);
        }
    }
}

#[test]
fn vertical_selection_remembers_column_across_short_lines() {
    let mut document = DocumentEditor::from_text("abcdef\nx\nabcdef");
    document.set_cursor(5, false);
    document.apply_edit_command(EditCommand::MoveDown { selecting: true });
    assert_eq!(document.cursor_line_column(), (1, 1));
    document.apply_edit_command(EditCommand::MoveDown { selecting: true });
    assert_eq!(document.cursor_line_column(), (2, 5));
    assert_eq!(document.selected_text(), Some("f\nx\nabcde"));
    document.apply_edit_command(EditCommand::MoveUp { selecting: true });
    document.apply_edit_command(EditCommand::MoveUp { selecting: true });
    assert_eq!(document.cursor(), 5);
    assert_eq!(document.selection(), None);
}

#[test]
fn shift_click_then_drag_preserves_the_original_selection_anchor() {
    let mut document = DocumentEditor::from_text("abcdef");
    document.set_cursor(2, false);
    document.extend_mouse_selection(5);
    document.update_mouse_selection(1);
    document.end_mouse_selection();
    assert_eq!(document.selected_text(), Some("b"));
    assert_eq!(document.cursor(), 1);
}

#[test]
fn document_editor_can_be_tested_without_a_workspace() {
    let mut document = DocumentEditor::from_text("hello");
    document.apply_edit_command(EditCommand::MoveLeft { selecting: true });
    document.apply_edit_command(EditCommand::Insert("p".into()));
    assert_eq!(document.text(), "hellp");
    assert!(document.is_dirty());
}

#[test]
fn document_editor_continues_numbered_lists() {
    let mut document = DocumentEditor::from_text("");
    document.insert_text("1. First");
    document.apply_edit_command(EditCommand::Newline);
    assert_eq!(document.text(), "1. First\n2. ");
}
