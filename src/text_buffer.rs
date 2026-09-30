use std::ops::Range;

/// Contiguous UTF-8 text with a line count updated only from each edit.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TextBuffer {
    text: String,
    line_breaks: usize,
}

impl TextBuffer {
    pub(crate) fn as_str(&self) -> &str {
        &self.text
    }

    pub(crate) fn line_count(&self) -> usize {
        self.line_breaks + 1
    }

    pub(crate) fn replace(&mut self, range: Range<usize>, replacement: &str) {
        // Slicing validates the range and UTF-8 boundaries before any mutation.
        let removed = line_breaks(&self.text[range.clone()]);
        let inserted = line_breaks(replacement);
        // ponytail: middle edits shift the suffix; use a balanced buffer if large notes require it.
        self.text.replace_range(range, replacement);
        self.line_breaks = self.line_breaks - removed + inserted;
    }
}

impl From<&str> for TextBuffer {
    fn from(text: &str) -> Self {
        Self {
            text: text.to_owned(),
            line_breaks: line_breaks(text),
        }
    }
}

fn line_breaks(text: &str) -> usize {
    text.bytes().filter(|&byte| byte == b'\n').count()
}

#[cfg(test)]
#[path = "tests/text_buffer.rs"]
mod tests;
