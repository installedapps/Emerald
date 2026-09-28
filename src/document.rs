use crate::parser::{ParseReport, ParsedDocument};
use crate::text_buffer::TextBuffer;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EditCommand {
    Insert(String),
    Newline,
    Backspace,
    Delete,
    MoveLeft { selecting: bool },
    MoveRight { selecting: bool },
    MoveUp { selecting: bool },
    MoveDown { selecting: bool },
    MoveLineStart { selecting: bool },
    MoveLineEnd { selecting: bool },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selection {
    pub start: usize,
    pub end: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct DocumentEditor {
    text: TextBuffer,
    parsed: ParsedDocument,
    parse_dirty: bool,
    parse_revision: u64,
    text_revision: u64,
    dirty: bool,
    cursor: usize,
    selection_anchor: Option<usize>,
    drag_anchor: Option<usize>,
    preferred_column: Option<usize>,
}

impl DocumentEditor {
    pub(crate) fn from_text(text: &str) -> Self {
        let text = TextBuffer::from(text);
        let parsed = ParsedDocument::from_source(text.as_str());
        Self::from_text_and_parsed(text, parsed)
    }

    pub(crate) fn from_text_and_parsed(text: TextBuffer, parsed: ParsedDocument) -> Self {
        let cursor = text.as_str().len();

        Self {
            text,
            parsed,
            parse_dirty: false,
            parse_revision: 1,
            text_revision: 1,
            dirty: false,
            cursor,
            selection_anchor: None,
            drag_anchor: None,
            preferred_column: None,
        }
    }

    pub(crate) fn text(&self) -> &str {
        self.text.as_str()
    }
    pub(crate) fn line_count(&self) -> usize {
        self.text.line_count()
    }
    pub(crate) fn parse_report(&self) -> &ParseReport {
        &self.parsed.report
    }

    pub(crate) fn parsed_document(&self) -> &ParsedDocument {
        &self.parsed
    }
    pub(crate) fn parse_is_dirty(&self) -> bool {
        self.parse_dirty
    }
    pub(crate) fn parse_revision(&self) -> u64 {
        self.parse_revision
    }
    pub(crate) fn text_revision(&self) -> u64 {
        self.text_revision
    }
    pub(crate) fn is_dirty(&self) -> bool {
        self.dirty
    }
    pub(crate) fn cursor(&self) -> usize {
        self.cursor
    }

    pub(crate) fn selection(&self) -> Option<Selection> {
        let anchor = self.selection_anchor?;
        (anchor != self.cursor).then_some(Selection {
            start: anchor.min(self.cursor),
            end: anchor.max(self.cursor),
        })
    }

    pub(crate) fn selected_text(&self) -> Option<&str> {
        let selection = self.selection()?;
        self.text.as_str().get(selection.start..selection.end)
    }

    pub(crate) fn set_cursor(&mut self, cursor: usize, selecting: bool) {
        self.preferred_column = None;
        if selecting && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = self.clamp_to_boundary(cursor);
        if !selecting {
            self.selection_anchor = None;
        }
    }

    pub(crate) fn select_all(&mut self) {
        self.preferred_column = None;
        self.selection_anchor = Some(0);
        self.cursor = self.text.as_str().len();
    }

    pub(crate) fn cursor_line_column(&self) -> (usize, usize) {
        line_column_for_offset(self.text.as_str(), self.cursor)
    }

    pub(crate) fn byte_offset_for_line_column(&self, line: usize, column: usize) -> usize {
        byte_offset_for_line_column(self.text.as_str(), line, column)
    }

    pub(crate) fn begin_mouse_selection(&mut self, cursor: usize) {
        self.preferred_column = None;
        let cursor = self.clamp_to_boundary(cursor);
        self.cursor = cursor;
        self.selection_anchor = Some(cursor);
        self.drag_anchor = Some(cursor);
    }

    pub(crate) fn extend_mouse_selection(&mut self, cursor: usize) {
        self.set_cursor(cursor, true);
        self.drag_anchor = self.selection_anchor;
    }

    pub(crate) fn update_mouse_selection(&mut self, cursor: usize) {
        let cursor = self.clamp_to_boundary(cursor);
        if self.drag_anchor.is_none() {
            self.drag_anchor = Some(self.cursor);
            self.selection_anchor = Some(self.cursor);
        }
        self.cursor = cursor;
    }

    pub(crate) fn end_mouse_selection(&mut self) {
        self.drag_anchor = None;
        if self.selection().is_none() {
            self.selection_anchor = None;
        }
    }

    pub(crate) fn apply_edit_command(&mut self, command: EditCommand) {
        match command {
            EditCommand::Insert(text) => self.insert_text(&text),
            EditCommand::Newline => self.insert_newline(),
            EditCommand::Backspace => self.backspace(),
            EditCommand::Delete => self.delete(),
            EditCommand::MoveLeft { selecting } => self.move_cursor_left(selecting),
            EditCommand::MoveRight { selecting } => self.move_cursor_right(selecting),
            EditCommand::MoveUp { selecting } => self.move_cursor_vertical(-1, selecting),
            EditCommand::MoveDown { selecting } => self.move_cursor_vertical(1, selecting),
            EditCommand::MoveLineStart { selecting } => {
                self.move_cursor_to_line_edge(false, selecting)
            }
            EditCommand::MoveLineEnd { selecting } => {
                self.move_cursor_to_line_edge(true, selecting)
            }
        }
    }

    pub(crate) fn insert_text(&mut self, text: &str) {
        if let Some(selection) = self.selection() {
            self.text.replace(selection.start..selection.end, text);
            self.cursor = selection.start + text.len();
            self.selection_anchor = None;
        } else {
            self.text.replace(self.cursor..self.cursor, text);
            self.cursor += text.len();
        }
        self.mark_changed();
    }

    pub(crate) fn backspace(&mut self) {
        if let Some(selection) = self.selection() {
            self.text.replace(selection.start..selection.end, "");
            self.cursor = selection.start;
            self.selection_anchor = None;
            self.mark_changed();
        } else if let Some(start) = previous_boundary(self.text.as_str(), self.cursor) {
            self.text.replace(start..self.cursor, "");
            self.cursor = start;
            self.mark_changed();
        }
    }

    pub(crate) fn delete(&mut self) {
        if let Some(selection) = self.selection() {
            self.text.replace(selection.start..selection.end, "");
            self.cursor = selection.start;
            self.selection_anchor = None;
            self.mark_changed();
        } else if let Some(end) = next_boundary(self.text.as_str(), self.cursor) {
            self.text.replace(self.cursor..end, "");
            self.mark_changed();
        }
    }

    pub(crate) fn refresh_parse_report_if_dirty(&mut self) -> bool {
        if !self.parse_dirty {
            return false;
        }
        self.parsed = ParsedDocument::from_source(self.text.as_str());
        self.parse_dirty = false;
        self.parse_revision = self.parse_revision.wrapping_add(1);
        true
    }

    pub(crate) fn apply_parse_result(
        &mut self,
        text_revision: u64,
        parsed: ParsedDocument,
    ) -> bool {
        if !self.parse_dirty || self.text_revision != text_revision {
            return false;
        }
        self.parsed = parsed;
        self.parse_dirty = false;
        self.parse_revision = self.parse_revision.wrapping_add(1);
        true
    }

    pub(crate) fn replace_loaded_text(&mut self, text: &str, parsed: ParsedDocument) {
        let text_revision = self.text_revision.wrapping_add(1);
        *self = Self::from_text_and_parsed(TextBuffer::from(text), parsed);
        self.text_revision = text_revision;
    }

    pub(crate) fn mark_clean(&mut self) {
        self.dirty = false;
    }

    fn mark_changed(&mut self) {
        self.preferred_column = None;
        self.dirty = true;
        self.text_revision = self.text_revision.wrapping_add(1);
        self.parse_dirty = true;
    }

    fn insert_newline(&mut self) {
        let line_start = self
            .text
            .as_str()
            .get(..self.cursor)
            .and_then(|text| text.rfind('\n'))
            .map_or(0, |offset| offset + 1);
        let line = &self.text.as_str()[line_start..self.cursor];
        if let Some((marker, empty)) = list_continuation(line) {
            if empty {
                self.text.replace(line_start..self.cursor, "\n");
                self.cursor = line_start + 1;
            } else {
                self.text
                    .replace(self.cursor..self.cursor, &format!("\n{marker}"));
                self.cursor += marker.len() + 1;
            }
            self.mark_changed();
        } else {
            self.insert_text("\n");
        }
    }

    fn move_cursor_left(&mut self, selecting: bool) {
        self.preferred_column = None;
        if !selecting {
            if let Some(selection) = self.selection() {
                self.set_cursor(selection.start, false);
                return;
            }
        }
        self.begin_selection_if_needed(selecting);
        if let Some(previous) = previous_boundary(self.text.as_str(), self.cursor) {
            self.cursor = previous;
        }
        if !selecting {
            self.selection_anchor = None;
        }
    }

    fn move_cursor_right(&mut self, selecting: bool) {
        self.preferred_column = None;
        if !selecting {
            if let Some(selection) = self.selection() {
                self.set_cursor(selection.end, false);
                return;
            }
        }
        self.begin_selection_if_needed(selecting);
        if let Some(next) = next_boundary(self.text.as_str(), self.cursor) {
            self.cursor = next;
        }
        if !selecting {
            self.selection_anchor = None;
        }
    }

    fn move_cursor_vertical(&mut self, delta: isize, selecting: bool) {
        let (line, column) = self.cursor_line_column();
        let column = self.preferred_column.unwrap_or(column);
        let target_line = if delta < 0 {
            line.saturating_sub(delta.unsigned_abs())
        } else {
            (line + delta as usize).min(self.text.line_count().saturating_sub(1))
        };
        self.set_cursor(
            self.byte_offset_for_line_column(target_line, column),
            selecting,
        );
        self.preferred_column = Some(column);
    }

    fn move_cursor_to_line_edge(&mut self, end: bool, selecting: bool) {
        let (line, _) = self.cursor_line_column();
        let column = if end {
            line_len_chars(self.text.as_str(), line)
        } else {
            0
        };
        self.set_cursor(self.byte_offset_for_line_column(line, column), selecting);
    }

    fn begin_selection_if_needed(&mut self, selecting: bool) {
        if selecting && self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
    }

    fn clamp_to_boundary(&self, cursor: usize) -> usize {
        let cursor = cursor.min(self.text.as_str().len());
        if self.text.as_str().is_char_boundary(cursor) {
            return cursor;
        }
        self.text
            .as_str()
            .char_indices()
            .map(|(index, _)| index)
            .take_while(|index| *index < cursor)
            .last()
            .unwrap_or(0)
    }
}

fn line_column_for_offset(text: &str, offset: usize) -> (usize, usize) {
    let mut line = 0;
    let mut column = 0;
    for (index, ch) in text.char_indices() {
        if index >= offset.min(text.len()) {
            break;
        }
        if ch == '\n' {
            line += 1;
            column = 0;
        } else {
            column += 1;
        }
    }
    (line, column)
}

fn byte_offset_for_line_column(text: &str, target_line: usize, target_column: usize) -> usize {
    let mut line = 0;
    let mut column = 0;
    for (index, ch) in text.char_indices() {
        if line == target_line && column == target_column {
            return index;
        }
        if ch == '\n' {
            if line == target_line {
                return index;
            }
            line += 1;
            column = 0;
        } else if line == target_line {
            column += 1;
        }
    }
    text.len()
}

fn line_len_chars(text: &str, target_line: usize) -> usize {
    text.lines()
        .nth(target_line)
        .map_or(0, |line| line.chars().count())
}

fn previous_boundary(text: &str, cursor: usize) -> Option<usize> {
    text.char_indices()
        .map(|(index, _)| index)
        .take_while(|index| *index < cursor)
        .last()
}

fn next_boundary(text: &str, cursor: usize) -> Option<usize> {
    text.char_indices()
        .map(|(index, _)| index)
        .find(|index| *index > cursor)
        .or_else(|| (cursor < text.len()).then_some(text.len()))
}

fn list_continuation(line_before_cursor: &str) -> Option<(String, bool)> {
    let indent_len = line_before_cursor.len() - line_before_cursor.trim_start().len();
    let indent = &line_before_cursor[..indent_len];
    let content = &line_before_cursor[indent_len..];
    if let Some(rest) = content.strip_prefix("* ") {
        return Some((format!("{indent}* "), rest.trim().is_empty()));
    }
    if let Some(rest) = content.strip_prefix(". ") {
        return Some((format!("{indent}. "), rest.trim().is_empty()));
    }
    let digits_len = content.chars().take_while(char::is_ascii_digit).count();
    if digits_len > 0
        && content
            .get(digits_len..)
            .is_some_and(|rest| rest.starts_with(". "))
    {
        let number = content[..digits_len].parse::<usize>().ok()?;
        let marker = format!("{indent}{}. ", number.saturating_add(1));
        let rest = &content[digits_len + 2..];
        return Some((marker, rest.trim().is_empty()));
    }
    None
}

#[cfg(test)]
mod tests {
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
}
