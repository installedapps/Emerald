use super::*;

#[test]
fn creates_a_default_workspace_with_a_visible_file() {
    let temp = tempfile::tempdir().unwrap();

    let state = EditorState::open_or_create(temp.path()).unwrap();

    assert_eq!(state.workspace_root(), temp.path());
    assert_eq!(state.files().len(), 1);
    assert_eq!(state.active_file().file_name().unwrap(), "welcome.adoc");
    assert!(state.text().contains("Emerald"));
    assert!(state.active_file().exists());
}

#[test]
fn loading_placeholder_defers_workspace_file_listing() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("note.adoc"), "= Note").unwrap();

    let state = EditorState::loading_placeholder(temp.path()).unwrap();

    assert!(state.files().is_empty());
    assert!(state.text().is_empty());
    assert!(state.active_file().exists());
}

#[test]
fn clicking_a_missing_adoc_link_can_create_it_in_the_workspace() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.insert_text("\nxref:linked.adoc[Linked]");

    let graph = state.link_graph().clone();
    let linked = temp.path().join("linked.adoc");
    assert!(graph.nodes.contains(&linked));
    assert!(!linked.exists());

    state.ensure_linked_file(&linked).unwrap();
    assert!(linked.exists());
    assert!(state.files().contains(&linked));
}

#[test]
fn graph_tracks_unsaved_links_aliases_and_link_removal() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    let source = state.active_file().to_path_buf();
    let target = temp.path().join("project notes.adoc");
    std::fs::write(&target, "= Project notes").unwrap();
    // Refresh workspace membership through the same boundary as link navigation.
    state.ensure_linked_file(&target).unwrap();
    assert!(state.link_graph().edges.is_empty());

    state.insert_text(
        "\n[[project notes#Plan|Plan]] and xref:project notes.adoc[Project] and [[missing]]",
    );
    let graph = state.link_graph().clone();
    assert_eq!(graph.edges.len(), 2);
    assert!(graph.edges.contains(&graph::GraphEdge {
        source: source.clone(),
        target: target.clone(),
    }));
    let missing = temp.path().join("missing.adoc");
    assert!(graph.nodes.contains(&missing));
    assert!(!missing.exists());
    assert!(!std::fs::read_to_string(&source).unwrap().contains("[["));
    assert!(graph.errors.is_empty());

    state.select_all();
    state.insert_text("No links now.");
    let graph = state.link_graph();
    assert!(graph.edges.is_empty());
    assert!(graph.nodes.contains(&target));
    assert!(!graph.nodes.contains(&missing));
}

#[test]
fn missing_non_adoc_link_is_not_created() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    assert!(state
        .ensure_linked_file(temp.path().join("bad.txt"))
        .is_err());
    assert!(!temp.path().join("bad.txt").exists());
}

#[test]
fn editing_text_updates_the_parse_report() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    state.insert_text("\n\n== First note\n\nSome body text.");

    assert!(state.text().contains("First note"));
    assert!(state.parse_is_dirty());
    assert!(state.refresh_parse_report_if_dirty());
    assert!(state.parse_report().parsed);
    assert_eq!(state.parse_report().diagnostic_count, 0);
    assert!(!state.parse_is_dirty());
}

#[test]
fn repeated_edits_mark_parse_dirty_without_reparsing_immediately() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    let revision = state.parse_revision();

    state.insert_text("a");
    state.insert_text("b");

    assert!(state.parse_is_dirty());
    assert_eq!(state.parse_revision(), revision);
    assert!(state.refresh_parse_report_if_dirty());
    assert!(state.parse_revision() > revision);
    assert!(!state.refresh_parse_report_if_dirty());
}

#[test]
fn text_revision_changes_only_when_document_content_changes() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    let revision = state.text_revision();

    state.set_cursor(0, false);
    assert_eq!(state.text_revision(), revision);

    state.insert_text("x");
    assert!(state.text_revision() > revision);
}

#[test]
fn backspace_removes_the_last_character() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    state.insert_text("x");
    state.backspace();

    assert!(!state.text().ends_with('x'));
}

#[test]
fn save_persists_the_current_document() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    state.insert_text("\nPersisted line.");
    state.save().unwrap();

    let saved = std::fs::read_to_string(state.active_file()).unwrap();
    assert!(saved.contains("Persisted line."));
}

#[test]
fn select_file_loads_another_asciidoc_document() {
    let temp = tempfile::tempdir().unwrap();
    let second = temp.path().join("zsecond.adoc");
    std::fs::write(&second, "= Second\n\nDifferent note.").unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    state.select_file(&second).unwrap();

    assert_eq!(state.active_file(), second);
    assert!(state.text().contains("Different note."));
}

#[test]
fn select_file_reuses_the_loaded_parse_report() {
    let temp = tempfile::tempdir().unwrap();
    let second = temp.path().join("second.adoc");
    std::fs::write(&second, "= Second\n\nDifferent note.").unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    let revision = state.parse_revision();

    state.select_file(&second).unwrap();

    assert!(state.parse_revision() >= revision);
    assert!(!state.parse_is_dirty());
    assert_eq!(state.parse_report().diagnostic_count, 0);
}

#[test]
fn edit_command_inserts_text_and_newlines_then_saves() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    state.apply_edit_command(EditCommand::Insert(" typed".to_string()));
    state.apply_edit_command(EditCommand::Newline);
    state.apply_edit_command(EditCommand::Insert("|===\n|A |B\n|1 |2\n|===".to_string()));
    state.save().unwrap();

    let saved = std::fs::read_to_string(state.active_file()).unwrap();
    assert!(saved.contains(" typed\n|==="));
    assert!(saved.contains("|1 |2"));
}

#[test]
fn enter_continues_unordered_lists() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("unordered").unwrap();

    state.insert_text("* First");
    state.apply_edit_command(EditCommand::Newline);

    assert_eq!(state.text(), "* First\n* ");
}

#[test]
fn enter_continues_and_numbers_numbered_lists() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("numbered").unwrap();

    state.insert_text("1. First");
    state.apply_edit_command(EditCommand::Newline);

    assert_eq!(state.text(), "1. First\n2. ");
}

#[test]
fn enter_twice_exits_a_list_without_leaving_a_marker() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("exit list").unwrap();

    state.insert_text("* First");
    state.apply_edit_command(EditCommand::Newline);
    state.apply_edit_command(EditCommand::Newline);

    assert_eq!(state.text(), "* First\n\n");
}

#[test]
fn cursor_moves_left_and_right_by_character_boundary() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("cursor").unwrap();
    state.apply_edit_command(EditCommand::Insert("abc".to_string()));

    state.apply_edit_command(EditCommand::MoveLeft { selecting: false });
    assert_eq!(state.cursor(), 2);
    state.apply_edit_command(EditCommand::MoveRight { selecting: false });
    assert_eq!(state.cursor(), 3);
}

#[test]
fn shift_arrow_creates_a_selection_and_selected_text() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("selection").unwrap();
    state.apply_edit_command(EditCommand::Insert("hello".to_string()));

    state.apply_edit_command(EditCommand::MoveLeft { selecting: true });
    state.apply_edit_command(EditCommand::MoveLeft { selecting: true });

    assert_eq!(state.selection(), Some(Selection { start: 3, end: 5 }));
    assert_eq!(state.selected_text(), Some("lo"));
}

#[test]
fn inserting_text_replaces_the_current_selection() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("selection replace").unwrap();
    state.apply_edit_command(EditCommand::Insert("hello".to_string()));
    state.apply_edit_command(EditCommand::MoveLeft { selecting: true });
    state.apply_edit_command(EditCommand::MoveLeft { selecting: true });

    state.apply_edit_command(EditCommand::Insert("p".to_string()));

    assert_eq!(state.text(), "help");
    assert_eq!(state.cursor(), 4);
    assert_eq!(state.selection(), None);
}

#[test]
fn delete_removes_the_character_after_the_cursor() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("delete").unwrap();
    state.apply_edit_command(EditCommand::Insert("abcd".to_string()));
    state.apply_edit_command(EditCommand::MoveLeft { selecting: false });
    state.apply_edit_command(EditCommand::MoveLeft { selecting: false });

    state.apply_edit_command(EditCommand::Delete);

    assert_eq!(state.text(), "abd");
    assert_eq!(state.cursor(), 2);
}

#[test]
fn vertical_cursor_movement_preserves_column_when_possible() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("vertical cursor").unwrap();
    state.apply_edit_command(EditCommand::Insert("abcde\nxy\n12345".to_string()));
    state.set_cursor(3, false);

    state.apply_edit_command(EditCommand::MoveDown { selecting: false });
    assert_eq!(state.cursor_line_column(), (1, 2));

    state.apply_edit_command(EditCommand::MoveDown { selecting: false });
    assert_eq!(state.cursor_line_column(), (2, 3));
}

#[test]
fn home_and_end_move_within_the_current_line() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("line edges").unwrap();
    state.apply_edit_command(EditCommand::Insert("first\nsecond".to_string()));
    state.set_cursor(state.byte_offset_for_line_column(1, 3), false);

    state.apply_edit_command(EditCommand::MoveLineStart { selecting: false });
    assert_eq!(state.cursor_line_column(), (1, 0));

    state.apply_edit_command(EditCommand::MoveLineEnd { selecting: false });
    assert_eq!(state.cursor_line_column(), (1, 6));
}

#[test]
fn mouse_line_click_mapping_preserves_utf8_boundaries() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("utf8 mouse placement").unwrap();
    state.apply_edit_command(EditCommand::Insert("one\n🦀 café\ntwo".to_string()));

    let after_crab = state.byte_offset_for_line_column(1, 1);
    let after_cafe = state.byte_offset_for_line_column(1, 6);

    assert_eq!(&state.text()[after_crab..], " café\ntwo");
    assert_eq!(&state.text()[after_cafe..], "\ntwo");
    assert!(state.text().is_char_boundary(after_crab));
    assert!(state.text().is_char_boundary(after_cafe));
}

#[test]
fn mouse_drag_selection_tracks_anchor_and_cursor() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("mouse selection").unwrap();
    state.apply_edit_command(EditCommand::Insert("abcdef".to_string()));

    state.begin_mouse_selection(1);
    state.update_mouse_selection(4);
    state.end_mouse_selection();

    assert_eq!(state.cursor(), 4);
    assert_eq!(state.selection(), Some(Selection { start: 1, end: 4 }));
    assert_eq!(state.selected_text(), Some("bcd"));
}

#[test]
fn mouse_drag_selection_can_extend_backwards_across_lines() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("backwards mouse selection").unwrap();
    state.apply_edit_command(EditCommand::Insert("first\nsecond".to_string()));

    let start = state.byte_offset_for_line_column(1, 4);
    let end = state.byte_offset_for_line_column(0, 1);
    state.begin_mouse_selection(start);
    state.update_mouse_selection(end);
    state.end_mouse_selection();

    assert_eq!(state.cursor(), end);
    assert_eq!(
        state.selection(),
        Some(Selection {
            start: end,
            end: start
        })
    );
    assert_eq!(state.selected_text(), Some("irst\nseco"));
}

#[test]
fn empty_mouse_click_clears_selection() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.create_file("mouse selection clear").unwrap();
    state.apply_edit_command(EditCommand::Insert("abcdef".to_string()));
    state.begin_mouse_selection(1);
    state.update_mouse_selection(4);
    state.end_mouse_selection();

    state.begin_mouse_selection(2);
    state.end_mouse_selection();

    assert_eq!(state.cursor(), 2);
    assert_eq!(state.selection(), None);
}

#[test]
fn new_file_creates_selects_and_saves_a_unique_asciidoc_file() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    state.create_file("daily note").unwrap();
    state.apply_edit_command(EditCommand::Insert("= Daily note".to_string()));
    state.save().unwrap();

    assert_eq!(state.active_file().file_name().unwrap(), "daily-note.adoc");
    assert!(state.files().iter().any(|file| {
        file.file_name()
            .is_some_and(|name| name == "daily-note.adoc")
    }));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("daily-note.adoc")).unwrap(),
        "= Daily note"
    );
}

#[test]
fn new_file_uses_a_numeric_suffix_when_name_exists() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("daily-note.adoc"), "= Existing").unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    state.create_file("daily note").unwrap();

    assert_eq!(
        state.active_file().file_name().unwrap(),
        "daily-note-2.adoc"
    );
}

#[test]
fn save_marks_the_document_clean_after_edits() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    assert!(!state.is_dirty());
    state.apply_edit_command(EditCommand::Insert("dirty".to_string()));
    assert!(state.is_dirty());
    state.save().unwrap();
    assert!(!state.is_dirty());
}

#[test]
fn long_editing_session_preserves_text_and_lines_across_save() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.select_all();
    state.insert_text("");
    for _ in 0..10_000 {
        for text in ["h", "é", "🙂", "\n", "x"] {
            state.apply_edit_command(EditCommand::Insert(text.into()));
        }
    }
    let expected = "hé🙂\nx".repeat(10_000);
    assert_eq!(state.text(), expected);
    assert_eq!(state.line_count(), 10_001);
    let revision = state.text_revision();
    state.save().unwrap();
    assert_eq!(
        std::fs::read_to_string(state.active_file()).unwrap(),
        expected
    );
    assert_eq!(state.text_revision(), revision);
    assert_eq!(state.cursor(), expected.len());
    assert_eq!(state.line_count(), 10_001);
    state.apply_edit_command(EditCommand::Backspace);
    state.insert_text("after save\n");
    assert_eq!(
        state.text(),
        format!("{}after save\n", &expected[..expected.len() - 1])
    );
    assert_eq!(state.line_count(), 10_002);
    assert!(state.is_dirty());
}

#[test]
fn selecting_a_file_saves_pending_edits_instead_of_losing_them() {
    let temp = tempfile::tempdir().unwrap();
    let second = temp.path().join("zsecond.adoc");
    std::fs::write(&second, "second").unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.insert_text("preserved");

    state.select_file(&second).unwrap();

    assert_eq!(
        std::fs::read_to_string(temp.path().join("welcome.adoc")).unwrap(),
        "= Emerald\n\nStart writing AsciiDoc here.\npreserved"
    );
    assert_eq!(state.text(), "second");
}

#[test]
fn selecting_a_file_outside_the_workspace_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let outside = tempfile::NamedTempFile::new().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    assert!(state.select_file(outside.path()).is_err());
}

#[test]
fn selecting_a_parent_directory_file_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let outside = temp.path().join("outside.adoc");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::write(&outside, "outside").unwrap();
    let mut state = EditorState::open_or_create(&workspace).unwrap();

    assert!(state.select_file("../outside.adoc").is_err());
}

#[cfg(unix)]
#[test]
fn selecting_a_symlink_to_a_file_outside_the_workspace_is_rejected() {
    use std::os::unix::fs::symlink;

    let temp = tempfile::tempdir().unwrap();
    let workspace = temp.path().join("workspace");
    let outside = temp.path().join("outside.adoc");
    let link = workspace.join("linked.adoc");
    std::fs::create_dir(&workspace).unwrap();
    std::fs::write(&outside, "outside").unwrap();
    symlink(&outside, &link).unwrap();
    let mut state = EditorState::open_or_create(&workspace).unwrap();

    assert!(state.select_file(&link).is_err());
}

#[test]
fn deleting_active_file_removes_it_and_selects_a_remaining_file() {
    let temp = tempfile::tempdir().unwrap();
    let second = temp.path().join("second.adoc");
    std::fs::write(&second, "second").unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    let deleted = state.active_file().to_path_buf();

    state.delete_active_file().unwrap();

    assert!(!deleted.exists());
    assert_eq!(state.active_file(), temp.path().join("welcome.adoc"));
    assert_eq!(state.text(), "= Emerald\n\nStart writing AsciiDoc here.\n");
    assert_eq!(state.files(), &[temp.path().join("welcome.adoc")]);
}

#[test]
fn deleting_the_last_workspace_file_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    assert!(state.delete_active_file().is_err());
    assert!(state.active_file().exists());
}

#[test]
fn renaming_active_file_updates_disk_and_sidebar_state() {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    state.rename_active_file("project notes").unwrap();

    assert_eq!(
        state.active_file().file_name().unwrap(),
        "project notes.adoc"
    );
    assert!(state.active_file().exists());
    assert!(!temp.path().join("welcome.adoc").exists());
    assert!(state.files().contains(&state.active_file().to_path_buf()));
}

#[test]
fn renaming_rejects_path_traversal_and_existing_names() {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("z-other.adoc"), "other").unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();

    assert!(state.rename_active_file("../outside").is_err());
    assert!(state.rename_active_file("z-other.adoc").is_err());
}
