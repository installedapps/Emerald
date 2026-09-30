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
#[path = "tests/slash_commands.rs"]
mod tests;
