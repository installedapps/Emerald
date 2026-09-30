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
    Image {
        target: String,
        alt: String,
        width: Option<u32>,
    },
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
    Image,
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
            Self::Image { .. } => (RenderBlockKind::Image, false),
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

/// Converts source into the parsed representation consumed by preview stages.
pub trait DocumentParser: Send + Sync {
    fn parse(&self, source: &str) -> ParsedDocument;
}

/// Converts parsed content into the blocks understood by the preview UI.
pub trait BlockRenderer: Send + Sync {
    fn render(
        &self,
        source: &str,
        parsed: &ParsedDocument,
        options: &RenderOptions,
    ) -> Vec<RenderBlock>;
}

/// Precomputes content-dependent data reused while a snapshot is displayed.
pub trait PreviewPreparer: Send + Sync {
    fn prepare(&self, blocks: &[RenderBlock]) -> crate::preview::PreparedPreview;
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct RenderOptions {
    pub toc: TocPolicy,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum TocPolicy {
    /// Follow the `:toc:` attribute in the source.
    #[default]
    Source,
    Disabled,
    Enabled,
}

/// Coordinates parsing, block rendering, and preview preparation.
///
/// Implementations are injected at the document stage boundaries, keeping the
/// pipeline independent of GPUI and workspace identity.
#[derive(Clone)]
pub struct PreviewPipeline {
    parser: Arc<dyn DocumentParser>,
    renderer: Arc<dyn BlockRenderer>,
    preparer: Arc<dyn PreviewPreparer>,
}

impl PreviewPipeline {
    pub fn new(
        parser: Arc<dyn DocumentParser>,
        renderer: Arc<dyn BlockRenderer>,
        preparer: Arc<dyn PreviewPreparer>,
    ) -> Self {
        Self {
            parser,
            renderer,
            preparer,
        }
    }

    pub fn parse(&self, source: &str) -> ParsedDocument {
        self.parser.parse(source)
    }

    pub fn snapshot(&self, source: &str, parsed: &ParsedDocument) -> RenderSnapshot {
        self.snapshot_with_options(source, parsed, &RenderOptions::default())
    }

    pub fn snapshot_with_options(
        &self,
        source: &str,
        parsed: &ParsedDocument,
        options: &RenderOptions,
    ) -> RenderSnapshot {
        let blocks = Arc::new(self.renderer.render(source, parsed, options));
        RenderSnapshot {
            report: parsed.report.clone(),
            rows: Arc::new(crate::preview::PreviewRow::prepare(&blocks)),
            prepared: Arc::new(self.preparer.prepare(&blocks)),
            blocks,
            style: RenderStyle::from_source(source),
        }
    }
}

impl Default for PreviewPipeline {
    fn default() -> Self {
        Self::new(
            Arc::new(AsciiDocParser),
            Arc::new(AsciiDocBlockRenderer),
            Arc::new(DefaultPreviewPreparer),
        )
    }
}

impl RenderSnapshot {
    pub fn from_source(source: &str) -> Self {
        let pipeline = PreviewPipeline::default();
        let parsed = pipeline.parse(source);
        pipeline.snapshot(source, &parsed)
    }

    pub fn from_parsed(source: &str, parsed: ParsedDocument) -> Self {
        PreviewPipeline::default().snapshot(source, &parsed)
    }
}

#[derive(Default)]
pub struct AsciiDocParser;

impl DocumentParser for AsciiDocParser {
    fn parse(&self, source: &str) -> ParsedDocument {
        ParsedDocument::from_source(source)
    }
}

#[derive(Default)]
pub struct AsciiDocBlockRenderer;

impl BlockRenderer for AsciiDocBlockRenderer {
    fn render(
        &self,
        source: &str,
        parsed: &ParsedDocument,
        options: &RenderOptions,
    ) -> Vec<RenderBlock> {
        let include_toc = match options.toc {
            TocPolicy::Source => has_table_of_contents(source),
            TocPolicy::Disabled => false,
            TocPolicy::Enabled => true,
        };
        render_parsed_blocks(&parsed.blocks, include_toc)
    }
}

#[derive(Default)]
pub struct DefaultPreviewPreparer;

impl PreviewPreparer for DefaultPreviewPreparer {
    fn prepare(&self, blocks: &[RenderBlock]) -> crate::preview::PreparedPreview {
        crate::preview::PreparedPreview::new(blocks)
    }
}

struct CachedRender {
    content_revision: u64,
    document_revision: Option<u64>,
    snapshot: RenderSnapshot,
}

pub struct RenderCache {
    entries: std::collections::HashMap<PathBuf, CachedRender>,
    pipeline: PreviewPipeline,
}

impl Default for RenderCache {
    fn default() -> Self {
        Self::with_pipeline(PreviewPipeline::default())
    }
}

impl RenderCache {
    pub fn with_pipeline(pipeline: PreviewPipeline) -> Self {
        Self {
            entries: std::collections::HashMap::new(),
            pipeline,
        }
    }

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
                            snapshot: self.pipeline.snapshot(source, parsed),
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
        self.insert(source_file, source, self.pipeline.snapshot(source, &parsed));
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
                    snapshot: {
                        let parsed = self.pipeline.parse(source);
                        self.pipeline.snapshot(source, &parsed)
                    },
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
    let parsed = ParsedDocument::from_source(source);
    AsciiDocBlockRenderer.render(source, &parsed, &RenderOptions::default())
}

fn render_parsed_blocks(parsed_blocks: &[ParsedBlock], include_toc: bool) -> Vec<RenderBlock> {
    let mut blocks = parsed_blocks
        .into_iter()
        .map(|b| match b {
            ParsedBlock::Heading { level, text } => RenderBlock::Heading {
                level: *level,
                text: text.clone(),
            },
            ParsedBlock::Paragraph(t) => RenderBlock::Paragraph(t.clone()),
            ParsedBlock::Image { target, alt, width } => RenderBlock::Image {
                target: target.clone(),
                alt: alt.clone(),
                width: *width,
            },
            ParsedBlock::ThematicBreak => RenderBlock::ThematicBreak,
            ParsedBlock::UnorderedList(v) => RenderBlock::UnorderedList(v.clone()),
            ParsedBlock::OrderedList(v) => RenderBlock::OrderedList(v.clone()),
            ParsedBlock::Quote(t) => RenderBlock::Quote(t.clone()),
            ParsedBlock::Admonition { kind, blocks } => RenderBlock::Admonition {
                kind: kind.clone(),
                blocks: render_parsed_blocks(blocks, false),
            },
            ParsedBlock::Table(v) => RenderBlock::Table(v.clone()),
            ParsedBlock::Code { language, code } => RenderBlock::Code {
                language: language.clone(),
                code: code.clone(),
            },
            ParsedBlock::Diagram {
                kind,
                title,
                source,
            } => RenderBlock::Diagram {
                kind: kind.clone(),
                title: title.clone(),
                source: source.clone(),
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
#[path = "tests/rendered.rs"]
mod tests;
