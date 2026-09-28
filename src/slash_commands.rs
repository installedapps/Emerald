//! AsciiDoc snippets and slash-search state, independent of GPUI.
use std::ops::Range;

#[derive(Clone, Copy, Debug)]
pub struct SlashCommand {
    pub label: &'static str,
    pub syntax: &'static str,
    pub placeholder: &'static str,
}

pub const COMMANDS: &[SlashCommand] = &[
    SlashCommand {
        label: "Table",
        syntax:
            "[cols=\"1,1\",options=\"header\"]\n|===\n|Column 1 |Column 2\n\n|Cell 1 |Cell 2\n|===",
        placeholder: "Column 1",
    },
    SlashCommand {
        label: "Callout",
        syntax: "[source]\n----\ncode // <1>\n----\n<1> Explanation",
        placeholder: "code",
    },
    SlashCommand {
        label: "Code",
        syntax: "[source,rust]\n----\ncode\n----",
        placeholder: "code",
    },
    SlashCommand {
        label: "Math",
        syntax: "[latexmath]\n++++\nx^2\n++++",
        placeholder: "x^2",
    },
    SlashCommand {
        label: "Embed",
        syntax: "include::file.adoc[]",
        placeholder: "file.adoc",
    },
    SlashCommand {
        label: "Attachment",
        syntax: "link:file.pdf[Attachment]",
        placeholder: "file.pdf",
    },
    SlashCommand {
        label: "Property",
        syntax: ":name: value",
        placeholder: "name",
    },
    SlashCommand {
        label: "Notes (Admonition)",
        syntax: "NOTE: Note text",
        placeholder: "Note text",
    },
];

#[derive(Clone, Debug)]
pub struct SlashSearch {
    pub range: Range<usize>,
    pub matches: Vec<usize>,
    pub selected: usize,
}

impl SlashSearch {
    pub fn at(text: &str, cursor: usize) -> Option<Self> {
        let before = text.get(..cursor)?;
        let start = before.rfind('/')?;
        if before[..start]
            .chars()
            .next_back()
            .is_some_and(|c| !c.is_whitespace())
        {
            return None;
        }
        let query = &before[start + 1..];
        if !query.chars().all(char::is_alphabetic) {
            return None;
        }
        let query = query.to_lowercase();
        let matches = COMMANDS
            .iter()
            .enumerate()
            .filter_map(|(index, command)| {
                let label = command.label.to_lowercase();
                let mut letters = label.chars();
                query
                    .chars()
                    .all(|letter| letters.by_ref().any(|c| c == letter))
                    .then_some(index)
            })
            .collect();
        Some(Self {
            range: start..cursor,
            matches,
            selected: 0,
        })
    }

    pub fn navigate(&mut self, backwards: bool) {
        let count = self.matches.len();
        if count > 0 {
            self.selected = (self.selected + if backwards { count - 1 } else { 1 }) % count;
        }
    }

    /// Isolate block syntax from surrounding prose without removing that prose.
    pub fn replacement(&self, text: &str, command: SlashCommand) -> (String, Range<usize>) {
        let before = &text[..self.range.start];
        let after = &text[self.range.end..];
        let prefix = if before.is_empty() || before.ends_with("\n\n") {
            ""
        } else if before.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        let suffix = if after.starts_with("\n\n") {
            ""
        } else if after.starts_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        let start =
            self.range.start + prefix.len() + command.syntax.find(command.placeholder).unwrap();
        (
            format!("{prefix}{}{suffix}", command.syntax),
            start..start + command.placeholder.len(),
        )
    }
}

#[cfg(test)]
mod tests {
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
}
