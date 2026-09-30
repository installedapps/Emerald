//! Reusable source measurements; cursor, selection and screen positions stay outside the cache.
use super::*;
use gpui::{ShapedLine, TextRun};
use std::{collections::VecDeque, ops::Range};

const MAX_LINES: usize = 128;
const MAX_TEXT_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub(super) struct SourceRowLayout {
    pub range: Range<usize>,
    pub last: bool,
    pub newline: bool,
    pub shaped: Rc<ShapedLine>,
    pub stops: Arc<[(usize, f32)]>,
}

/// Owned by one file/revision/wrap-width snapshot. Source font and size are fixed.
#[derive(Default)]
pub(super) struct SourceLayoutCache {
    // ponytail: bounded FIFO is enough for nearby scrolling; no general-purpose cache dependency.
    lines: VecDeque<(usize, usize, Rc<Vec<SourceRowLayout>>)>,
    text_bytes: usize,
}

impl SourceLayoutCache {
    pub fn get_or_prepare(
        &mut self,
        index: usize,
        text: &str,
        start: usize,
        width: f32,
        text_color: u32,
        window: &mut Window,
    ) -> Rc<Vec<SourceRowLayout>> {
        if let Some((_, _, rows)) = self.lines.iter().find(|(key, _, _)| *key == index) {
            return rows.clone();
        }
        let rows = Rc::new(prepare(text, start, width, text_color, window));
        if text.len() <= MAX_TEXT_BYTES {
            while self.lines.len() >= MAX_LINES || self.text_bytes + text.len() > MAX_TEXT_BYTES {
                let (_, bytes, _) = self.lines.pop_front().unwrap();
                self.text_bytes -= bytes;
            }
            self.text_bytes += text.len();
            self.lines.push_back((index, text.len(), rows.clone()));
        }
        rows
    }
}

fn prepare(
    text: &str,
    start: usize,
    width: f32,
    color: u32,
    window: &mut Window,
) -> Vec<SourceRowLayout> {
    let rows = emerald::ui::wrapped_source_line(text, start, usize::MAX, None, |text| {
        let shaped = window.text_system().shape_text(
            text.to_owned().into(),
            px(18.0),
            &[text_run(text.len(), color)],
            Some(px(width)),
            None,
        );
        match shaped {
            Ok(lines) => lines.first().map_or_else(Vec::new, |line| {
                line.wrap_boundaries
                    .iter()
                    .map(|boundary| {
                        line.unwrapped_layout.runs[boundary.run_ix].glyphs[boundary.glyph_ix].index
                    })
                    .collect()
            }),
            Err(error) => {
                tracing::warn!(%error, "could not wrap source line");
                Vec::new()
            }
        }
    });
    let count = rows.len();
    rows.into_iter()
        .enumerate()
        .map(|(index, row)| {
            let content = &text[row.start - start..row.end - start];
            let shaped = window.text_system().shape_line(
                content.to_owned().into(),
                px(18.0),
                &[text_run(content.len(), color)],
                None,
            );
            let stops = character_stops(content, &shaped);
            SourceRowLayout {
                range: row.start..row.end,
                last: index + 1 == count,
                newline: index + 1 == count && text.ends_with('\n'),
                shaped: Rc::new(shaped),
                stops: stops.into(),
            }
        })
        .collect()
}

fn text_run(len: usize, color: u32) -> TextRun {
    TextRun {
        len,
        font: gpui::font(SOURCE_FONT_FAMILY),
        color: rgb(color).into(),
        background_color: None,
        underline: None,
        strikethrough: None,
    }
}

fn character_stops(text: &str, shaped: &ShapedLine) -> Vec<(usize, f32)> {
    // Match GPUI's first-glyph-at-or-after lookup, but traverse glyphs only once.
    let mut glyphs = shaped.runs.iter().flat_map(|run| &run.glyphs).peekable();
    text.char_indices()
        .map(|(index, _)| index)
        .chain(std::iter::once(text.len()))
        .map(|index| {
            while glyphs.peek().is_some_and(|glyph| glyph.index < index) {
                glyphs.next();
            }
            let x = glyphs.peek().map_or(shaped.width, |glyph| glyph.position.x);
            (index, f32::from(x))
        })
        .collect()
}

#[cfg(test)]
#[path = "tests/source_layout.rs"]
mod tests;
