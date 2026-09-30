use asciidork_ast::{
    AttrData, Block, BlockContent, BlockContext, Cell, CellContent, DocContent, Document, Inline,
    InlineNodes, ListVariant, MultiAttrList,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseReport {
    pub parsed: bool,
    pub diagnostic_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParsedBlock {
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
        blocks: Vec<ParsedBlock>,
    },
    Table(Vec<Vec<String>>),
    Code {
        language: Option<String>,
        code: String,
    },
    Diagram {
        kind: String,
        title: Option<String>,
        source: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedDocument {
    pub report: ParseReport,
    pub blocks: Vec<ParsedBlock>,
}

/// A source-level note reference shared by preview links and the workspace graph.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InlineReference {
    pub start: usize,
    pub end: usize,
    pub target: String,
    pub label: String,
    pub wiki: bool,
}

impl InlineReference {
    /// Resolve the optional extension identically for preview navigation and graph edges.
    pub fn file_target(&self) -> String {
        if self.wiki && !self.target.ends_with(".adoc") {
            format!("{}.adoc", self.target)
        } else {
            self.target.clone()
        }
    }
}

pub fn extract_references(source: &str) -> Vec<InlineReference> {
    let mut references = Vec::new();
    let mut offset = 0;
    while offset < source.len() {
        let rest = &source[offset..];
        let xref = rest.find("xref:").map(|index| (index, false));
        let wiki = rest.find("[[").map(|index| (index, true));
        let Some((relative, is_wiki)) =
            [xref, wiki].into_iter().flatten().min_by_key(|item| item.0)
        else {
            break;
        };
        let start = offset + relative;
        let (target, label, end) = if is_wiki {
            let content_start = start + 2;
            let Some(close) = source[content_start..].find("]]") else {
                break;
            };
            let raw = source[content_start..content_start + close].trim();
            let (target, label) = raw.split_once('|').unwrap_or((raw, raw));
            (
                target.trim().to_string(),
                label.trim().to_string(),
                content_start + close + 2,
            )
        } else {
            let target_start = start + 5;
            let Some(label_start) = source[target_start..].find('[') else {
                offset = target_start;
                continue;
            };
            let label_start = target_start + label_start;
            let Some(label_end) = source[label_start + 1..].find(']') else {
                offset = label_start + 1;
                continue;
            };
            let label_end = label_start + 1 + label_end;
            (
                source[target_start..label_start].trim().to_string(),
                source[label_start + 1..label_end].to_string(),
                label_end + 1,
            )
        };
        let target = target.split('#').next().unwrap_or_default().trim();
        if !target.is_empty() {
            references.push(InlineReference {
                start,
                end,
                target: target.to_string(),
                label,
                wiki: is_wiki,
            });
        }
        offset = end.max(start + 1);
    }
    references
}

impl ParsedDocument {
    pub fn from_source(source: &str) -> Self {
        let arena = asciidork_parser::prelude::Bump::new();
        let parser = asciidork_parser::Parser::from_str(
            source,
            asciidork_parser::prelude::SourceFile::Tmp,
            &arena,
        );
        // Preview editing is intentionally tolerant: Asciidork's strict mode
        // turns recoverable document diagnostics (such as an unresolved xref
        // while its target note is being edited) into a total parse failure.
        // Keep those diagnostics in the report while retaining the AST.
        let mut parser = parser;
        let mut settings = asciidork_core::JobSettings::embedded();
        settings.strict = false;
        parser.apply_job_settings(settings);

        let parsed = parser.parse();
        let result = match parsed {
            Ok(result) => Self {
                report: ParseReport {
                    parsed: true,
                    diagnostic_count: result.warnings.len(),
                },
                blocks: document_blocks(&result.document),
            },
            Err(diagnostics) => Self {
                report: ParseReport {
                    parsed: false,
                    diagnostic_count: diagnostics.len(),
                },
                blocks: fallback_blocks(source),
            },
        };
        if !result.report.parsed {
            tracing::warn!(
                diagnostics = result.report.diagnostic_count,
                "AsciiDoc parser rejected the document; using fallback preview"
            );
        } else if result.report.diagnostic_count > 0 {
            tracing::debug!(
                diagnostics = result.report.diagnostic_count,
                "AsciiDoc parser reported warnings"
            );
        }
        result
    }
}

impl ParseReport {
    pub fn from_source(source: &str) -> Self {
        ParsedDocument::from_source(source).report
    }
}

fn document_blocks(document: &Document<'_>) -> Vec<ParsedBlock> {
    let mut blocks = Vec::new();
    if let Some(title) = document.title() {
        blocks.push(ParsedBlock::Heading {
            level: 1,
            text: inline_text(&title.main),
        });
    }

    match &document.content {
        DocContent::Blocks(items) => extend_blocks(items, &mut blocks),
        DocContent::Sections(content) => {
            if let Some(preamble) = &content.preamble {
                extend_blocks(preamble, &mut blocks);
            }
            for section in &content.sections {
                blocks.push(ParsedBlock::Heading {
                    level: section.level as usize + 1,
                    text: inline_text(&section.heading),
                });
                extend_blocks(&section.blocks, &mut blocks);
            }
        }
        DocContent::Parts(_) => {}
    }
    blocks
}

/// Recover the block structure needed by the preview when the full parser
/// rejects otherwise useful source. This intentionally stays small: the
/// Asciidork parse remains authoritative whenever it succeeds.
fn fallback_blocks(source: &str) -> Vec<ParsedBlock> {
    let mut blocks = Vec::new();
    let mut paragraph = Vec::new();

    let flush_paragraph = |paragraph: &mut Vec<&str>, blocks: &mut Vec<ParsedBlock>| {
        if !paragraph.is_empty() {
            blocks.push(ParsedBlock::Paragraph(
                paragraph
                    .iter()
                    .map(|line| fallback_inline_text(line))
                    .collect::<Vec<_>>()
                    .join(" "),
            ));
            paragraph.clear();
        }
    };

    for line in source.lines() {
        let trimmed = line.trim();
        let marker_count = trimmed
            .chars()
            .take_while(|character| *character == '=')
            .count();
        if marker_count > 0
            && trimmed
                .get(marker_count..)
                .is_some_and(|rest| rest.starts_with(' '))
        {
            flush_paragraph(&mut paragraph, &mut blocks);
            blocks.push(ParsedBlock::Heading {
                level: marker_count,
                text: fallback_inline_text(trimmed[marker_count..].trim()),
            });
        } else if trimmed.is_empty() {
            flush_paragraph(&mut paragraph, &mut blocks);
        } else if !trimmed.starts_with(':') {
            paragraph.push(trimmed);
        }
    }
    flush_paragraph(&mut paragraph, &mut blocks);
    blocks
}

fn fallback_inline_text(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = [rest.find("xref:"), rest.find("[[")]
        .into_iter()
        .flatten()
        .min()
    {
        output.push_str(&rest[..start]);
        if rest[start..].starts_with("[[") {
            let link = &rest[start + 2..];
            let Some(end) = link.find("]]") else {
                output.push_str(&rest[start..]);
                return output;
            };
            output.push_str(&rest[start..start + end + 4]);
            rest = &link[end + 2..];
        } else {
            let link = &rest[start + 5..];
            let Some(label_start) = link.find('[') else {
                output.push_str("xref:");
                rest = link;
                continue;
            };
            let Some(label_end) = link[label_start + 1..].find(']') else {
                output.push_str("xref:");
                rest = link;
                continue;
            };
            let label_end = label_start + 1 + label_end;
            // Keep the macro in the preview text so the UI can render its
            // label as a clickable note link while retaining the target.
            output.push_str(&rest[start..start + 5]);
            output.push_str(&link[..label_end + 1]);
            rest = &link[label_end + 1..];
        }
    }
    output.push_str(rest);
    output
}

fn extend_blocks(items: &[Block<'_>], output: &mut Vec<ParsedBlock>) {
    for block in items {
        let admonition_kind = match block.context {
            BlockContext::AdmonitionCaution => Some("CAUTION"),
            BlockContext::AdmonitionImportant => Some("IMPORTANT"),
            BlockContext::AdmonitionNote => Some("NOTE"),
            BlockContext::AdmonitionTip => Some("TIP"),
            BlockContext::AdmonitionWarning => Some("WARNING"),
            _ => None,
        };
        if let Some(kind) = admonition_kind {
            let mut blocks = Vec::new();
            match &block.content {
                BlockContent::Compound(children) => extend_blocks(children, &mut blocks),
                BlockContent::Simple(nodes) => {
                    blocks.push(ParsedBlock::Paragraph(inline_text(nodes)));
                }
                _ => {}
            }
            output.push(ParsedBlock::Admonition {
                kind: kind.into(),
                blocks,
            });
            continue;
        }
        match &block.content {
            BlockContent::Compound(children) => {
                if block.context == BlockContext::BlockQuote {
                    let text = children
                        .iter()
                        .filter_map(|child| match &child.content {
                            BlockContent::Simple(nodes) => Some(inline_text(nodes)),
                            _ => None,
                        })
                        .collect::<Vec<_>>()
                        .join(" ");
                    if !text.is_empty() {
                        output.push(ParsedBlock::Quote(text));
                    }
                } else {
                    extend_blocks(children, output);
                }
            }
            BlockContent::Section(section) => {
                output.push(ParsedBlock::Heading {
                    level: section.level as usize + 1,
                    text: inline_text(&section.heading),
                });
                extend_blocks(&section.blocks, output);
            }
            BlockContent::Simple(nodes) => match block.context {
                BlockContext::Listing | BlockContext::Literal => {
                    let kind = block.meta.attrs.str_positional_at(0).unwrap_or("");
                    if is_diagram_kind(kind) {
                        output.push(ParsedBlock::Diagram {
                            kind: kind.to_ascii_lowercase(),
                            title: block.meta.attrs.str_positional_at(1).map(ToOwned::to_owned),
                            source: inline_source(nodes).replace(" - ", " -> "),
                        });
                    } else {
                        output.push(ParsedBlock::Code {
                            language: source_language(&block.meta.attrs),
                            code: inline_source(nodes),
                        });
                    }
                }
                BlockContext::BlockQuote | BlockContext::QuotedParagraph => {
                    output.push(ParsedBlock::Quote(inline_text(nodes)))
                }
                _ => output.push(ParsedBlock::Paragraph(inline_text(nodes))),
            },
            BlockContent::QuotedParagraph { quote, .. } => {
                output.push(ParsedBlock::Quote(inline_text(quote)))
            }
            BlockContent::List { variant, items, .. } => {
                let values = items
                    .iter()
                    .map(|item| inline_text(&item.principle))
                    .collect();
                match variant {
                    ListVariant::Ordered => output.push(ParsedBlock::OrderedList(values)),
                    ListVariant::Unordered => output.push(ParsedBlock::UnorderedList(values)),
                    _ if items.first().is_some_and(|item| {
                        matches!(
                            item.marker,
                            asciidork_ast::ListMarker::Digits(_)
                                | asciidork_ast::ListMarker::Dot(_)
                        )
                    }) =>
                    {
                        output.push(ParsedBlock::OrderedList(values))
                    }
                    _ => {}
                }
            }
            BlockContent::Table(table) => {
                let mut rows = Vec::new();
                if let Some(header) = &table.header_row {
                    rows.push(header.cells.iter().map(cell_text).collect());
                }
                rows.extend(
                    table
                        .rows
                        .iter()
                        .map(|row| row.cells.iter().map(cell_text).collect()),
                );
                output.push(ParsedBlock::Table(rows));
            }
            BlockContent::Empty(asciidork_ast::EmptyMetadata::Image { target, attrs, .. }) => {
                output.push(ParsedBlock::Image {
                    target: target.to_string(),
                    alt: attrs.str_positional_at(0).unwrap_or(target).to_string(),
                    width: attrs
                        .named("width")
                        .or_else(|| attrs.str_positional_at(1))
                        .and_then(|value| value.trim_end_matches("px").parse::<u32>().ok())
                        .filter(|width| *width > 0),
                });
            }
            BlockContent::Empty(_) if block.context == BlockContext::ThematicBreak => {
                output.push(ParsedBlock::ThematicBreak);
            }
            BlockContent::Empty(_) | BlockContent::DocumentAttribute(_, _) => {}
        }
    }
}

fn cell_text(cell: &Cell<'_>) -> String {
    match &cell.content {
        CellContent::AsciiDoc(document) => document_blocks(document)
            .into_iter()
            .map(|block| match block {
                ParsedBlock::Paragraph(text) | ParsedBlock::Heading { text, .. } => text,
                _ => String::new(),
            })
            .collect::<Vec<_>>()
            .join(" "),
        CellContent::Literal(nodes) => inline_source(nodes),
        CellContent::Default(nodes)
        | CellContent::Emphasis(nodes)
        | CellContent::Header(nodes)
        | CellContent::Monospace(nodes)
        | CellContent::Strong(nodes) => nodes.iter().map(inline_text).collect::<Vec<_>>().join(" "),
    }
}

fn source_language(attrs: &MultiAttrList<'_>) -> Option<String> {
    attrs.source_language().map(ToOwned::to_owned)
}

fn is_diagram_kind(kind: &str) -> bool {
    matches!(
        kind,
        "blockdiag" | "plantuml" | "mermaid" | "graphviz" | "dot" | "ditaa" | "seqdiag" | "actdiag"
    )
}

fn inline_text(nodes: &InlineNodes<'_>) -> String {
    nodes
        .iter()
        .map(|node| match &node.content {
            Inline::Macro(asciidork_ast::MacroNode::Xref {
                target, linktext, ..
            }) => format!(
                "xref:{}[{}]",
                &target[..],
                linktext
                    .as_ref()
                    .map(inline_text)
                    .unwrap_or_else(|| target.to_string())
            ),
            Inline::Macro(asciidork_ast::MacroNode::InlineImage { target, attrs, .. }) => {
                let alt = attrs.str_positional_at(0).unwrap_or(target);
                let width = attrs.named("width").or_else(|| attrs.str_positional_at(1));
                match width {
                    Some(width) => format!("image:{}[{alt},{width}]", &target[..]),
                    None => format!("image:{}[{alt}]", &target[..]),
                }
            }
            Inline::Span(_, _, children) | Inline::Quote(_, children) => inline_text(children),
            _ => node_text(node),
        })
        .collect()
}

fn node_text(node: &asciidork_ast::InlineNode<'_>) -> String {
    match &node.content {
        Inline::Newline => " ".into(),
        Inline::Text(text) => text.to_string(),
        Inline::Symbol(asciidork_ast::SymbolKind::SingleRightArrow) => "->".into(),
        Inline::Symbol(asciidork_ast::SymbolKind::DoubleRightArrow) => "=>".into(),
        Inline::InlinePassthru(children) | Inline::Span(_, _, children) => inline_text(children),
        Inline::SpecialChar(kind) => match kind {
            asciidork_ast::SpecialCharKind::Ampersand => "&".into(),
            asciidork_ast::SpecialCharKind::LessThan => "<".into(),
            asciidork_ast::SpecialCharKind::GreaterThan => ">".into(),
        },
        Inline::Symbol(kind) => match kind {
            asciidork_ast::SymbolKind::SingleRightArrow => "->".into(),
            asciidork_ast::SymbolKind::DoubleRightArrow => "=>".into(),
            asciidork_ast::SymbolKind::SingleLeftArrow => "<-".into(),
            asciidork_ast::SymbolKind::DoubleLeftArrow => "<=".into(),
            _ => String::new(),
        },
        Inline::MultiCharWhitespace(_) => " ".into(),
        Inline::CurlyQuote(kind) => match kind {
            asciidork_ast::CurlyKind::LeftDouble => "“".into(),
            asciidork_ast::CurlyKind::RightDouble => "”".into(),
            asciidork_ast::CurlyKind::LeftSingle => "‘".into(),
            asciidork_ast::CurlyKind::RightSingle
            | asciidork_ast::CurlyKind::LegacyImplicitApostrophe => "'".into(),
        },
        _ => String::new(),
    }
}

fn inline_source(nodes: &InlineNodes<'_>) -> String {
    nodes
        .iter()
        .map(|node| match &node.content {
            Inline::Newline => "\n".to_string(),
            Inline::Text(text) => text.to_string(),
            Inline::Symbol(asciidork_ast::SymbolKind::SingleRightArrow) => "->".to_string(),
            Inline::Symbol(asciidork_ast::SymbolKind::DoubleRightArrow) => "=>".to_string(),
            _ => String::new(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wiki_aliases_and_headings_resolve_to_the_note_file() {
        let source = "See [[daily note#Plan|Today's plan]] and [[two.adoc|Two]].";
        let references = extract_references(source);
        assert_eq!(references.len(), 2);
        assert_eq!(references[0].file_target(), "daily note.adoc");
        assert_eq!(references[0].label, "Today's plan");
        assert_eq!(
            &source[references[0].start..references[0].end],
            "[[daily note#Plan|Today's plan]]"
        );
        assert_eq!(references[1].file_target(), "two.adoc");
        assert_eq!(references[1].label, "Two");
    }

    #[test]
    fn parses_basic_asciidoc_with_asciidork() {
        let report = ParseReport::from_source("= Title\n\nA paragraph with *strong* text.");

        assert!(report.parsed);
        assert_eq!(report.diagnostic_count, 0);
    }

    #[test]
    fn parsed_document_is_the_single_source_for_diagnostics_and_content() {
        let document = ParsedDocument::from_source("= Title\n\nA *formatted* paragraph.");

        assert!(document.report.parsed);
        assert_eq!(document.report.diagnostic_count, 0);
        assert_eq!(
            document.blocks,
            vec![
                ParsedBlock::Heading {
                    level: 1,
                    text: "Title".into()
                },
                ParsedBlock::Paragraph("A formatted paragraph.".into())
            ]
        );
    }

    #[test]
    fn parses_notes_with_cross_references() {
        let document = ParsedDocument::from_source(
            "= Test 1\n\nThis is the first test note.\n\nGo to xref:test-2.adoc[Test 2].\n",
        );

        assert!(document.report.parsed);
        assert_eq!(document.report.diagnostic_count, 1);
        assert!(!document.blocks.is_empty());
        assert_eq!(
            document.blocks[0],
            ParsedBlock::Heading {
                level: 1,
                text: "Test 1".into(),
            }
        );
        assert_eq!(
            document.blocks[1],
            ParsedBlock::Paragraph("This is the first test note.".into())
        );
        assert_eq!(
            document.blocks[2],
            ParsedBlock::Paragraph("Go to xref:test-2.adoc[Test 2].".into())
        );
    }
}
