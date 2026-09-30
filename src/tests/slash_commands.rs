use super::*;

#[test]
fn notes_can_be_found_by_name_or_asciidoc_term() {
    for query in ["/note", "/notes", "/admonition"] {
        let search = SlashSearch::at(query, query.len()).unwrap();
        assert_eq!(search.matches.len(), 1);
        assert_eq!(COMMANDS[search.matches[0]].syntax, "NOTE: Note text");
    }
}

#[test]
fn triggers_only_at_word_boundaries_and_filters_fuzzily() {
    for text in ["/", "hello /", "界\n/"] {
        assert_eq!(SlashSearch::at(text, text.len()).unwrap().matches.len(), 8);
    }
    for text in ["https://", "path/", "/code ", "/co/de"] {
        assert!(SlashSearch::at(text, text.len()).is_none());
    }
    let mut search = SlashSearch::at("/CD", 3).unwrap();
    assert_eq!(search.matches, vec![2]);
    search.navigate(true);
    assert_eq!(search.selected, 0);
    let mut empty = SlashSearch::at("/xyz", 4).unwrap();
    empty.navigate(false);
    assert!(empty.matches.is_empty());
}

#[test]
fn navigation_wraps_and_backspacing_refilters_the_query() {
    let mut search = SlashSearch::at("/", 1).unwrap();
    search.navigate(true);
    assert_eq!(search.selected, 7);
    search.navigate(false);
    assert_eq!(search.selected, 0);
    assert_eq!(SlashSearch::at("/co", 3).unwrap().matches, vec![1, 2]);
    assert_eq!(SlashSearch::at("/c", 2).unwrap().matches, vec![1, 2, 5]);
    assert!(SlashSearch::at("", 0).is_none());
}

#[test]
fn blocks_have_blank_line_boundaries_at_start_middle_and_end() {
    for (source, cursor, expected) in [
        ("/", 1, "include::file.adoc[]\n\n"),
        ("before\n/", 8, "\ninclude::file.adoc[]\n\n"),
        ("before\n\n/\n\nafter", 9, "include::file.adoc[]"),
    ] {
        let search = SlashSearch::at(source, cursor).unwrap();
        assert_eq!(search.replacement(source, COMMANDS[4]).0, expected);
    }
}

#[test]
fn snippets_replace_search_preserve_prose_and_select_placeholder() {
    for command in COMMANDS {
        let mut editor = crate::document::DocumentEditor::from_text("界 /code trailing");
        let search = SlashSearch::at(editor.text(), "界 /code".len()).unwrap();
        let (text, placeholder) = search.replacement(editor.text(), *command);
        editor.set_cursor(search.range.start, false);
        editor.set_cursor(search.range.end, true);
        editor.insert_text(&text);
        editor.set_cursor(placeholder.start, false);
        editor.set_cursor(placeholder.end, true);
        assert_eq!(editor.selected_text(), Some(command.placeholder));
        assert!(editor.text().starts_with("界 \n\n"));
        assert!(editor.text().ends_with("\n\n trailing"));
        assert!(!editor.text().contains("/code"));
    }
}
