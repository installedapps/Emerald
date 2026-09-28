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
mod tests {
    use super::*;

    #[test]
    fn counts_empty_and_trailing_lines() {
        for (text, count) in [("", 1), ("hello", 1), ("\n", 2), ("a\r\nb\n", 3)] {
            assert_eq!(TextBuffer::from(text).line_count(), count);
        }
    }

    #[test]
    fn replaces_unicode_and_updates_line_count() {
        let mut buffer = TextBuffer::from("a🙂\nbé\n");
        buffer.replace(1..6, "é\n\n");
        assert_eq!(buffer.as_str(), "aé\n\nbé\n");
        assert_eq!(buffer.line_count(), 4);
        buffer.replace(3..5, "");
        assert_eq!(buffer.as_str(), "aébé\n");
        assert_eq!(buffer.line_count(), 2);
        buffer.replace(0..buffer.as_str().len(), "");
        assert_eq!(buffer, TextBuffer::from(""));
    }

    #[test]
    fn long_typing_session_matches_freshly_loaded_text() {
        let mut buffer = TextBuffer::from("");
        for _ in 0..10_000 {
            for text in ["h", "é", "🙂", "\n", "x"] {
                let end = buffer.as_str().len();
                buffer.replace(end..end, text);
            }
        }
        let expected = "hé🙂\nx".repeat(10_000);
        assert_eq!(buffer.as_str(), expected);
        assert_eq!(buffer.line_count(), 10_001);
        assert_eq!(buffer, TextBuffer::from(expected.as_str()));
    }

    #[test]
    fn mixed_edits_keep_cached_line_count_correct() {
        let mut buffer = TextBuffer::from("alpha🙂\nbeta\n");
        let mut expected = buffer.as_str().to_owned();
        let mut seed = 0x5eed_u64;
        for _ in 0..10_000 {
            seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
            let boundaries: Vec<_> = expected
                .char_indices()
                .map(|(i, _)| i)
                .chain(std::iter::once(expected.len()))
                .collect();
            let start = boundaries[seed as usize % boundaries.len()];
            seed = seed.rotate_left(17);
            let end = boundaries[seed as usize % boundaries.len()];
            let range = start.min(end)..start.max(end);
            let replacement = ["", "x", "🙂", "\n"][seed as usize % 4];
            expected.replace_range(range.clone(), replacement);
            buffer.replace(range, replacement);
            assert_eq!(buffer.as_str(), expected);
            assert_eq!(buffer.line_count(), expected.split('\n').count());
        }
    }

    #[test]
    fn invalid_ranges_leave_text_and_line_count_unchanged() {
        for range in [2..3, 0..99, Range { start: 5, end: 1 }] {
            let mut buffer = TextBuffer::from("a🙂\n");
            let original = buffer.clone();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                buffer.replace(range, "\n\n");
            }));
            assert!(result.is_err());
            assert_eq!(buffer, original);
        }
    }
}
