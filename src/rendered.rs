use crate::parser::{ParseReport, ParsedBlock, ParsedDocument};
use std::path::{Path, PathBuf};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RenderStyle {
    pub text: Option<u32>,
    pub muted_text: Option<u32>,
    pub accent: Option<u32>,
    pub border: Option<u32>,
    pub panel: Option<u32>,
    pub heading: Option<u32>,
    pub code_background: Option<u32>,
    pub table_background: Option<u32>,
    pub quote: Option<u32>,
}
#[derive(Clone, Debug, PartialEq)]
pub enum RenderBlock {
    TableOfContents(Vec<TocEntry>),
    Heading {
        level: usize,
        text: String,
    },
    Paragraph(String),
    ThematicBreak,
    UnorderedList(Vec<String>),
    OrderedList(Vec<String>),
    Quote(String),
    Admonition {
        kind: String,
        blocks: Vec<RenderBlock>,
    },
    Code {
        language: Option<String>,
        code: String,
    },
    Diagram {
        kind: String,
        title: Option<String>,
        source: String,
    },
    Table(Vec<Vec<String>>),
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TocEntry {
    pub level: usize,
    pub text: String,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RenderBlockKind {
    Text,
    ThematicBreak,
    List,
    Quote,
    Admonition,
    Code,
    Diagram,
    Table,
    TableOfContents,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RenderBlockLayout {
    pub kind: RenderBlockKind,
    pub wraps: bool,
}
impl RenderBlock {
    pub fn layout(&self) -> RenderBlockLayout {
        let (kind, wraps) = match self {
            Self::TableOfContents(_) => (RenderBlockKind::TableOfContents, true),
            Self::Heading { .. } | Self::Paragraph(_) => (RenderBlockKind::Text, true),
            Self::UnorderedList(_) | Self::OrderedList(_) => (RenderBlockKind::List, true),
            Self::ThematicBreak => (RenderBlockKind::ThematicBreak, false),
            Self::Quote(_) => (RenderBlockKind::Quote, true),
            Self::Admonition { .. } => (RenderBlockKind::Admonition, true),
            Self::Code { .. } => (RenderBlockKind::Code, false),
            Self::Diagram { .. } => (RenderBlockKind::Diagram, true),
            Self::Table(_) => (RenderBlockKind::Table, true),
        };
        RenderBlockLayout { kind, wraps }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct RenderSnapshot {
    pub report: ParseReport,
    pub blocks: Arc<Vec<RenderBlock>>,
    pub rows: Arc<Vec<crate::preview::PreviewRow>>,
    pub prepared: Arc<crate::preview::PreparedPreview>,
    pub style: RenderStyle,
}

impl RenderSnapshot {
    pub fn from_source(source: &str) -> Self {
        Self::from_parsed(source, ParsedDocument::from_source(source))
    }

    pub fn from_parsed(source: &str, parsed: ParsedDocument) -> Self {
        let blocks = render_parsed_blocks(parsed.blocks, has_table_of_contents(source));
        let rows = Arc::new(crate::preview::PreviewRow::prepare(&blocks));
        Self {
            prepared: Arc::new(crate::preview::PreparedPreview::new(&blocks)),
            report: parsed.report.clone(),
            blocks: Arc::new(blocks),
            rows,
            style: RenderStyle::from_source(source),
        }
    }
}

struct CachedRender {
    content_revision: u64,
    document_revision: Option<u64>,
    snapshot: RenderSnapshot,
}

#[derive(Default)]
pub struct RenderCache {
    entries: std::collections::HashMap<PathBuf, CachedRender>,
}
impl RenderCache {
    /// Reuse prepared blocks on interaction-only frames. A missing parse means
    /// edits are pending: keep the last preview instead of parsing during render.
    pub fn snapshot_for_revision(
        &mut self,
        source: &str,
        source_file: &Path,
        text_revision: u64,
        parsed: Option<&ParsedDocument>,
    ) -> Option<&RenderSnapshot> {
        if let Some(parsed) = parsed {
            let entry = self.entries.get(source_file);
            if entry.is_none_or(|entry| entry.document_revision != Some(text_revision)) {
                let revision = content_revision(source);
                if entry.is_none_or(|entry| entry.content_revision != revision) {
                    self.entries.insert(
                        source_file.to_path_buf(),
                        CachedRender {
                            content_revision: revision,
                            document_revision: Some(text_revision),
                            snapshot: RenderSnapshot::from_parsed(source, parsed.clone()),
                        },
                    );
                } else {
                    self.entries.get_mut(source_file).unwrap().document_revision =
                        Some(text_revision);
                }
            }
        }
        self.entries.get(source_file).map(|entry| &entry.snapshot)
    }

    pub fn style(&mut self, source: &str, source_file: &Path) -> RenderStyle {
        self.refresh(source, source_file).style
    }
    pub fn blocks_arc(&mut self, source: &str, source_file: &Path) -> Arc<Vec<RenderBlock>> {
        self.refresh(source, source_file).blocks.clone()
    }
    pub fn insert(&mut self, source_file: PathBuf, source: &str, snapshot: RenderSnapshot) {
        self.entries.insert(
            source_file,
            CachedRender {
                content_revision: content_revision(source),
                document_revision: None,
                snapshot,
            },
        );
    }

    pub fn insert_parsed(&mut self, source_file: PathBuf, source: &str, parsed: ParsedDocument) {
        self.insert(
            source_file,
            source,
            RenderSnapshot::from_parsed(source, parsed),
        );
    }
    fn refresh(&mut self, source: &str, source_file: &Path) -> &RenderSnapshot {
        let revision = content_revision(source);
        let needs_refresh = self
            .entries
            .get(source_file)
            .is_none_or(|entry| entry.content_revision != revision);
        if needs_refresh {
            self.entries.insert(
                source_file.to_path_buf(),
                CachedRender {
                    content_revision: revision,
                    document_revision: None,
                    snapshot: RenderSnapshot::from_source(source),
                },
            );
        }
        &self
            .entries
            .get(source_file)
            .expect("render cache entry")
            .snapshot
    }
}

fn content_revision(source: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    source.hash(&mut hasher);
    hasher.finish()
}
impl RenderStyle {
    pub fn from_source(source: &str) -> Self {
        let mut style = Self::default();
        let source_lower = source.to_ascii_lowercase();
        let Some(style_start) = source_lower.find("<style") else {
            return style;
        };

        let css_start = source[style_start..]
            .find('>')
            .map(|offset| style_start + offset + 1);
        let Some(css_start) = css_start else {
            return style;
        };
        let Some(css_end) = source_lower[css_start..].find("</style>") else {
            return style;
        };
        let css = &source[css_start..css_start + css_end];
        let mut variables = std::collections::HashMap::new();
        let rules = css.split('}').filter_map(|rule| rule.split_once('{'));
        let rules = rules.collect::<Vec<_>>();
        for (_, declarations) in &rules {
            for (name, value) in declarations.split(';').filter_map(|d| d.split_once(':')) {
                if name.trim().starts_with("--") {
                    variables.insert(name.trim().to_string(), value.trim().to_string());
                }
            }
        }

        for (selectors, declarations) in rules {
            for (name, raw_value) in declarations.split(';').filter_map(|d| d.split_once(':')) {
                let name = name.trim();
                let Some(color) = css_color(raw_value, &variables) else {
                    continue;
                };
                for selector in selectors.split(',').map(str::trim) {
                    apply_css_color(&mut style, selector, name, color);
                }
            }
        }
        style
    }
}

fn css_color(
    raw_value: &str,
    variables: &std::collections::HashMap<String, String>,
) -> Option<u32> {
    let mut value = raw_value.trim().trim_end_matches("!important").trim();
    if let Some(variable) = value.strip_prefix("var(").and_then(|v| v.strip_suffix(')')) {
        value = variables.get(variable.trim())?.trim();
    }
    let value = value.strip_prefix('#')?;
    let value = if value.len() == 3 {
        value.chars().flat_map(|c| [c, c]).collect::<String>()
    } else {
        value.to_string()
    };
    (value.len() == 6)
        .then(|| u32::from_str_radix(&value, 16).ok())
        .flatten()
}

fn apply_css_color(style: &mut RenderStyle, selector: &str, property: &str, color: u32) {
    let selector = selector.to_ascii_lowercase();
    match property {
        "color"
            if selector.contains("h1")
                || selector.contains("h2")
                || selector.contains("h3")
                || selector.contains("h4") =>
        {
            style.heading = Some(color)
        }
        "color" if selector.contains('a') => style.accent = Some(color),
        "color" if selector.contains("blockquote") => style.quote = Some(color),
        "color" => style.text = Some(color),
        "background" | "background-color"
            if selector.contains("pre") || selector.contains("code") =>
        {
            style.code_background = Some(color)
        }
        "background" | "background-color" if selector.contains("table") => {
            style.table_background = Some(color)
        }
        "background" | "background-color" => style.panel = Some(color),
        "border-color" | "border" => style.border = Some(color),
        _ => {}
    }
}
pub fn render_blocks(source: &str) -> Vec<RenderBlock> {
    render_parsed_blocks(
        ParsedDocument::from_source(source).blocks,
        has_table_of_contents(source),
    )
}

fn render_parsed_blocks(parsed_blocks: Vec<ParsedBlock>, include_toc: bool) -> Vec<RenderBlock> {
    let mut blocks = parsed_blocks
        .into_iter()
        .map(|b| match b {
            ParsedBlock::Heading { level, text } => RenderBlock::Heading { level, text },
            ParsedBlock::Paragraph(t) => RenderBlock::Paragraph(t),
            ParsedBlock::ThematicBreak => RenderBlock::ThematicBreak,
            ParsedBlock::UnorderedList(v) => RenderBlock::UnorderedList(v),
            ParsedBlock::OrderedList(v) => RenderBlock::OrderedList(v),
            ParsedBlock::Quote(t) => RenderBlock::Quote(t),
            ParsedBlock::Admonition { kind, blocks } => RenderBlock::Admonition {
                kind,
                blocks: render_parsed_blocks(blocks, false),
            },
            ParsedBlock::Table(v) => RenderBlock::Table(v),
            ParsedBlock::Code { language, code } => RenderBlock::Code { language, code },
            ParsedBlock::Diagram {
                kind,
                title,
                source,
            } => RenderBlock::Diagram {
                kind,
                title,
                source,
            },
        })
        .collect::<Vec<_>>();
    let headings = blocks
        .iter()
        .filter_map(|b| match b {
            RenderBlock::Heading { level, text } => Some(TocEntry {
                level: *level,
                text: text.clone(),
            }),
            _ => None,
        })
        .collect::<Vec<_>>();
    if include_toc && !headings.is_empty() {
        blocks.insert(0, RenderBlock::TableOfContents(headings));
    }
    blocks
}

fn has_table_of_contents(source: &str) -> bool {
    source.lines().any(|line| {
        let attribute = line.trim();
        attribute == ":toc:" || attribute.starts_with(":toc:")
    })
}
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiagramEdge {
    pub from: String,
    pub to: String,
}
pub fn diagram_edges(source: &str) -> Vec<DiagramEdge> {
    source
        .lines()
        .filter_map(|line| {
            let (from, to) = line.split_once("->").or_else(|| line.split_once(" - "))?;
            Some(DiagramEdge {
                from: from.trim().to_string(),
                to: to.trim().to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_inline_and_block_admonitions_with_their_contents() {
        for kind in ["NOTE", "TIP", "IMPORTANT", "WARNING", "CAUTION"] {
            let expected = RenderBlock::Admonition {
                kind: kind.into(),
                blocks: vec![RenderBlock::Paragraph("Some note text.".into())],
            };
            for source in [
                format!("{kind}: Some note text."),
                format!("[{kind}]\n====\nSome note text.\n===="),
            ] {
                assert_eq!(render_blocks(&source), vec![expected.clone()], "{source}");
            }
        }
        assert_eq!(
            render_blocks(
                "[NOTE]\n====\nFirst paragraph.\n\nSecond paragraph.\n\n* One\n* Two\n===="
            ),
            vec![RenderBlock::Admonition {
                kind: "NOTE".into(),
                blocks: vec![
                    RenderBlock::Paragraph("First paragraph.".into()),
                    RenderBlock::Paragraph("Second paragraph.".into()),
                    RenderBlock::UnorderedList(vec!["One".into(), "Two".into()]),
                ],
            }]
        );
    }

    #[test]
    fn renders_thematic_breaks_between_paragraphs() {
        for marker in ["'''", "---", "***", "- - -", "* * *"] {
            assert_eq!(
                render_blocks(&format!("Before.\n\n{marker}\n\nAfter.")),
                vec![
                    RenderBlock::Paragraph("Before.".into()),
                    RenderBlock::ThematicBreak,
                    RenderBlock::Paragraph("After.".into()),
                ],
                "{marker}"
            );
        }
        assert!(matches!(
            render_blocks("----\ncode\n----").as_slice(),
            [RenderBlock::Code { .. }]
        ));
    }

    #[test]
    fn renders_headings_and_paragraphs_without_an_implicit_table_of_contents() {
        let blocks = render_blocks("= Title\n\nA paragraph.");
        assert!(matches!(blocks[0], RenderBlock::Heading { level: 1, .. }));
        assert!(matches!(blocks[1], RenderBlock::Paragraph(_)));
    }

    #[test]
    fn renders_an_explicit_table_of_contents_before_headings() {
        let blocks = render_blocks(":toc:\n\n= Title\n\nA paragraph.");
        assert!(matches!(blocks[0], RenderBlock::TableOfContents(_)));
        assert!(matches!(blocks[1], RenderBlock::Heading { level: 1, .. }));
        assert!(matches!(blocks[2], RenderBlock::Paragraph(_)));
    }

    #[test]
    fn renders_xref_notes_with_headings_and_paragraphs() {
        for (source, heading, paragraph, link_text) in [
            (
                "= Test 1\n\nThis is the first test note.\n\nGo to xref:test-2.adoc[Test 2].\n",
                "Test 1",
                "This is the first test note.",
                "Go to xref:test-2.adoc[Test 2].",
            ),
            (
                "= Test 2\n\nThis is the second test note.\n\nBack to xref:test-1.adoc[Test 1].\n",
                "Test 2",
                "This is the second test note.",
                "Back to xref:test-1.adoc[Test 1].",
            ),
        ] {
            let blocks = render_blocks(source);
            assert!(
                matches!(blocks[0], RenderBlock::Heading { level: 1, ref text } if text == heading)
            );
            assert!(matches!(blocks[1], RenderBlock::Paragraph(ref text) if text == paragraph));
            assert!(matches!(blocks[2], RenderBlock::Paragraph(ref text) if text == link_text));
        }
    }

    #[test]
    fn extracts_diagram_edges_for_the_preview() {
        let edges = diagram_edges("A -> B\nB -> C");
        assert_eq!(edges[0].from, "A");
        assert_eq!(edges[1].to, "C");
    }

    #[test]
    fn interaction_frames_reuse_blocks_and_pending_edits_keep_the_last_preview() {
        let mut cache = RenderCache::default();
        let file = Path::new("note.adoc");
        let source = "= Original";
        let parsed = ParsedDocument::from_source(source);
        let first = cache
            .snapshot_for_revision(source, file, 1, Some(&parsed))
            .unwrap()
            .blocks
            .clone();
        let prepared = cache
            .snapshot_for_revision(source, file, 1, Some(&parsed))
            .unwrap()
            .prepared
            .clone();
        for _ in 0..100 {
            let frame = cache
                .snapshot_for_revision(source, file, 1, Some(&parsed))
                .unwrap();
            assert!(Arc::ptr_eq(&first, &frame.blocks));
            assert!(Arc::ptr_eq(&prepared, &frame.prepared));
        }
        let pending = cache
            .snapshot_for_revision("= Changed", file, 2, None)
            .unwrap();
        assert!(Arc::ptr_eq(&first, &pending.blocks));
        assert!(Arc::ptr_eq(&prepared, &pending.prepared));
        let updated = ParsedDocument::from_source("= Changed");
        let ready = cache
            .snapshot_for_revision("= Changed", file, 2, Some(&updated))
            .unwrap();
        assert!(!Arc::ptr_eq(&first, &ready.blocks));
        assert!(!Arc::ptr_eq(&prepared, &ready.prepared));
        assert_eq!(ready.prepared.inline("Changed")[0].text.as_ref(), "Changed");
        assert!(matches!(&ready.blocks[0], RenderBlock::Heading { text, .. } if text == "Changed"));
        assert!(cache
            .snapshot_for_revision("", Path::new("other.adoc"), 3, None)
            .is_none());
    }

    #[test]
    fn revision_lookup_reuses_a_snapshot_prepared_during_file_loading() {
        let mut cache = RenderCache::default();
        let file = PathBuf::from("note.adoc");
        let source = "= Loaded";
        let parsed = ParsedDocument::from_source(source);
        let snapshot = RenderSnapshot::from_parsed(source, parsed.clone());
        let blocks = snapshot.blocks.clone();
        cache.insert(file.clone(), source, snapshot);
        let frame = cache
            .snapshot_for_revision(source, &file, 4, Some(&parsed))
            .unwrap();
        assert!(Arc::ptr_eq(&blocks, &frame.blocks));
    }

    #[test]
    fn keeps_rendered_snapshots_for_multiple_files() {
        let mut cache = RenderCache::default();
        let first = PathBuf::from("first.adoc");
        let second = PathBuf::from("second.adoc");

        let first_blocks = cache.blocks_arc("= First", &first);
        let _ = cache.blocks_arc("= Second", &second);
        let first_blocks_again = cache.blocks_arc("= First", &first);

        assert!(Arc::ptr_eq(&first_blocks, &first_blocks_again));
    }

    #[test]
    fn applies_each_note_css_palette_including_custom_properties() {
        let style = RenderStyle::from_source(
            r#"++++
<style>
:root { --note-bg: #101820; --note-text: #f2f4f8; --note-accent: #ffb86c; }
body { background: var(--note-bg); color: var(--note-text); }
h1, h2 { color: var(--note-accent) !important; }
</style>
++++"#,
        );

        assert_eq!(style.panel, Some(0x101820));
        assert_eq!(style.text, Some(0xf2f4f8));
        assert_eq!(style.heading, Some(0xffb86c));
    }

    #[test]
    fn notes_without_custom_css_leave_the_standard_theme_as_fallback() {
        assert_eq!(
            RenderStyle::from_source("= Plain note"),
            RenderStyle::default()
        );
    }
}
