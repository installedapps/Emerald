//! Bounded slices of compound blocks for the variable-height preview list.
use crate::parser::extract_references;
use crate::rendered::RenderBlock;
use crate::rendered::{diagram_edges, DiagramEdge};
use std::{collections::HashMap, ops::Range, sync::Arc};

pub const PREVIEW_CHUNK_SIZE: usize = 16;

#[derive(Clone, Debug, PartialEq)]
pub struct PreviewRow {
    pub block_index: usize,
    pub range: Range<usize>,
    pub first: bool,
    pub last: bool,
}

impl PreviewRow {
    pub fn prepare(blocks: &[RenderBlock]) -> Vec<Self> {
        let mut rows = Vec::new();
        for (block_index, block) in blocks.iter().enumerate() {
            let ranges = match block {
                RenderBlock::OrderedList(items) | RenderBlock::UnorderedList(items) => {
                    chunks(items.len())
                }
                RenderBlock::Table(items) => chunks(items.len()),
                RenderBlock::TableOfContents(items) => chunks(items.len()),
                RenderBlock::Code { code, .. } => {
                    // Store byte ranges once: scrolling near the bottom must not
                    // scan all preceding lines to reach the requested chunk.
                    let mut ranges = Vec::new();
                    let mut start = 0;
                    let mut end = 0;
                    for (index, line) in code.split_inclusive('\n').enumerate() {
                        end += line.len();
                        if (index + 1) % PREVIEW_CHUNK_SIZE == 0 {
                            ranges.push(start..end);
                            start = end;
                        }
                    }
                    if start < end {
                        ranges.push(start..end);
                    }
                    ranges
                }
                _ => Vec::new(),
            };
            let count = ranges.len();
            if count <= 1 {
                rows.push(Self {
                    block_index,
                    range: 0..0,
                    first: true,
                    last: true,
                });
            } else {
                rows.extend(ranges.into_iter().enumerate().map(|(index, range)| Self {
                    block_index,
                    range,
                    first: index == 0,
                    last: index + 1 == count,
                }));
            }
        }
        rows
    }

    /// Borrow the visible portion without copying its strings or nested rows.
    pub fn items<'a, T>(&self, items: &'a [T]) -> &'a [T] {
        if self.first && self.last {
            items
        } else {
            &items[self.range.clone()]
        }
    }

    pub fn text<'a>(&self, text: &'a str) -> &'a str {
        if self.first && self.last {
            text
        } else {
            &text[self.range.clone()]
        }
    }
}

fn chunks(len: usize) -> Vec<Range<usize>> {
    (0..len)
        .step_by(PREVIEW_CHUNK_SIZE)
        .map(|start| start..(start + PREVIEW_CHUNK_SIZE).min(len))
        .collect()
}

/// Content-dependent work shared by every frame of a render snapshot.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PreparedPreview {
    inline: HashMap<String, Vec<InlinePart>>,
    diagrams: HashMap<String, Vec<DiagramEdge>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InlinePart {
    pub text: Arc<str>,
    pub target: Option<Arc<str>>,
    pub image: Option<InlineImage>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct InlineImage {
    pub target: Arc<str>,
    pub width: Option<u32>,
}

impl PreparedPreview {
    pub fn new(blocks: &[RenderBlock]) -> Self {
        let mut prepared = Self::default();
        prepared.add_blocks(blocks);
        prepared
    }

    pub fn inline(&self, text: &str) -> &[InlinePart] {
        &self.inline[text]
    }

    pub fn edges(&self, source: &str) -> &[DiagramEdge] {
        &self.diagrams[source]
    }

    fn add_text(&mut self, text: &str) {
        self.inline.entry(text.to_owned()).or_insert_with(|| {
            let mut parts = Vec::new();
            let mut cursor = 0;
            let mut items = extract_references(text)
                .into_iter()
                .map(|reference| {
                    let target = reference.file_target();
                    (
                        reference.start,
                        reference.end,
                        reference.label,
                        Some(Arc::from(target)),
                        None,
                    )
                })
                .collect::<Vec<_>>();
            items.extend(extract_inline_images(text).into_iter().map(|image| {
                (
                    image.start,
                    image.end,
                    image.alt,
                    None,
                    Some(InlineImage {
                        target: image.target.into(),
                        width: image.width,
                    }),
                )
            }));
            items.sort_by_key(|item| item.0);
            for (start, end, label, target, image) in items {
                if start < cursor {
                    continue;
                }
                if start > cursor {
                    parts.push(InlinePart {
                        text: text[cursor..start].into(),
                        target: None,
                        image: None,
                    });
                }
                parts.push(InlinePart {
                    target,
                    text: label.into(),
                    image,
                });
                cursor = end;
            }
            if cursor < text.len() || parts.is_empty() {
                parts.push(InlinePart {
                    text: text[cursor..].into(),
                    target: None,
                    image: None,
                });
            }
            parts
        });
    }

    fn add_blocks(&mut self, blocks: &[RenderBlock]) {
        for block in blocks {
            match block {
                RenderBlock::Heading { text, .. }
                | RenderBlock::Paragraph(text)
                | RenderBlock::Quote(text) => self.add_text(text),
                RenderBlock::OrderedList(items) | RenderBlock::UnorderedList(items) => {
                    for item in items {
                        self.add_text(item);
                    }
                }
                RenderBlock::Table(rows) => {
                    for cell in rows.iter().flatten() {
                        self.add_text(cell);
                    }
                }
                RenderBlock::TableOfContents(entries) => {
                    for entry in entries {
                        self.add_text(&entry.text);
                    }
                }
                RenderBlock::Admonition { blocks, .. } => self.add_blocks(blocks),
                RenderBlock::Diagram { source, .. } => {
                    self.diagrams
                        .entry(source.clone())
                        .or_insert_with(|| diagram_edges(source));
                }
                RenderBlock::Code { .. }
                | RenderBlock::Image { .. }
                | RenderBlock::ThematicBreak => {}
            }
        }
    }
}

struct InlineImageMatch {
    start: usize,
    end: usize,
    target: String,
    alt: String,
    width: Option<u32>,
}

fn extract_inline_images(text: &str) -> Vec<InlineImageMatch> {
    let mut images = Vec::new();
    let mut offset = 0;
    while let Some(relative) = text[offset..].find("image:") {
        let start = offset + relative;
        let target_start = start + "image:".len();
        if text[target_start..].starts_with(':') {
            offset = target_start + 1;
            continue;
        }
        let Some(open) = text[target_start..].find('[').map(|at| target_start + at) else {
            break;
        };
        let Some(close) = text[open + 1..].find(']').map(|at| open + 1 + at) else {
            break;
        };
        let target = &text[target_start..open];
        if !target.is_empty() && !target.chars().any(char::is_whitespace) {
            let (alt, width) = text[open + 1..close]
                .split_once(',')
                .map_or((&text[open + 1..close], None), |(alt, width)| {
                    (alt, width.trim_end_matches("px").parse::<u32>().ok())
                });
            images.push(InlineImageMatch {
                start,
                end: close + 1,
                target: target.into(),
                alt: if alt.is_empty() { target } else { alt }.into(),
                width,
            });
        }
        offset = close + 1;
    }
    images
}

#[cfg(test)]
#[path = "tests/preview.rs"]
mod tests;
