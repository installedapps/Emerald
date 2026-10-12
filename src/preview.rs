//! Bounded slices of compound blocks for the variable-height preview list.
use crate::parser::extract_references;
use crate::rendered::RenderBlock;
use crate::rendered::{diagram_edges, DiagramEdge};
use std::{collections::HashMap, ops::Range, sync::Arc};
use tree_sitter::{Language, Node, Parser};

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
    code: HashMap<String, HashMap<String, Vec<CodeToken>>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CodeTokenKind {
    Plain,
    Keyword,
    String,
    Comment,
    Number,
    Function,
    Type,
    Constant,
    Property,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CodeToken {
    pub range: Range<usize>,
    pub kind: CodeTokenKind,
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

    pub fn code_tokens(&self, language: Option<&str>, source: &str) -> &[CodeToken] {
        self.code
            .get(source)
            .and_then(|languages| languages.get(language.unwrap_or_default()))
            .map(Vec::as_slice)
            .unwrap_or(&[])
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
                RenderBlock::Code { language, code } => {
                    self.code
                        .entry(code.clone())
                        .or_default()
                        .entry(language.clone().unwrap_or_default())
                        .or_insert_with(|| highlight_code(language.as_deref(), code));
                }
                RenderBlock::Image { .. } | RenderBlock::ThematicBreak => {}
            }
        }
    }
}

fn highlight_code(language: Option<&str>, source: &str) -> Vec<CodeToken> {
    let Some(language) = language.and_then(language_for) else {
        return vec![CodeToken {
            range: 0..source.len(),
            kind: CodeTokenKind::Plain,
        }];
    };
    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return vec![CodeToken {
            range: 0..source.len(),
            kind: CodeTokenKind::Plain,
        }];
    }
    let Some(tree) = parser.parse(source, None) else {
        return vec![CodeToken {
            range: 0..source.len(),
            kind: CodeTokenKind::Plain,
        }];
    };
    let mut leaves = Vec::new();
    collect_code_leaves(tree.root_node(), CodeTokenKind::Plain, source, &mut leaves);
    leaves.sort_by_key(|token| token.range.start);

    let mut tokens: Vec<CodeToken> = Vec::new();
    let mut cursor = 0;
    for leaf in leaves {
        let start = leaf.range.start.max(cursor);
        let end = leaf.range.end.min(source.len());
        if start > cursor {
            push_code_token(&mut tokens, cursor..start, CodeTokenKind::Plain);
        }
        if start < end {
            push_code_token(&mut tokens, start..end, leaf.kind);
            cursor = end;
        }
    }
    if cursor < source.len() {
        push_code_token(&mut tokens, cursor..source.len(), CodeTokenKind::Plain);
    }
    tokens
}

fn language_for(name: &str) -> Option<Language> {
    match name.to_ascii_lowercase().as_str() {
        "rust" | "rs" => Some(tree_sitter_rust::LANGUAGE.into()),
        "json" => Some(tree_sitter_json::LANGUAGE.into()),
        "nix" => Some(tree_sitter_nix::LANGUAGE.into()),
        "javascript" | "js" | "jsx" | "ecmascript" => Some(tree_sitter_javascript::LANGUAGE.into()),
        "c" | "h" => Some(tree_sitter_c::LANGUAGE.into()),
        "java" => Some(tree_sitter_java::LANGUAGE.into()),
        "python" | "py" => Some(tree_sitter_python::LANGUAGE.into()),
        "c#" | "csharp" | "c-sharp" | "cs" => Some(tree_sitter_c_sharp::LANGUAGE.into()),
        _ => None,
    }
}

fn collect_code_leaves(
    node: Node<'_>,
    inherited: CodeTokenKind,
    source: &str,
    leaves: &mut Vec<CodeToken>,
) {
    let kind = node.kind().parse_kind().unwrap_or(inherited);
    if node.child_count() == 0 {
        if node.start_byte() < node.end_byte() {
            let kind = if inherited == CodeTokenKind::String {
                inherited
            } else {
                classify_leaf(node, source).unwrap_or(kind)
            };
            leaves.push(CodeToken {
                range: node.start_byte()..node.end_byte(),
                kind,
            });
        }
        return;
    }

    let child_kind = match node.kind() {
        "string_literal" | "raw_string_literal" | "char_literal" | "string" | "string_content"
        | "string_fragment" | "quoted_string" => CodeTokenKind::String,
        "line_comment" | "block_comment" | "comment" => CodeTokenKind::Comment,
        "integer_literal"
        | "float_literal"
        | "number"
        | "number_literal"
        | "integer_expression"
        | "float_expression"
        | "integer"
        | "float"
        | "decimal_integer_literal"
        | "hex_integer_literal"
        | "binary_integer_literal"
        | "decimal_floating_point_literal"
        | "hex_floating_point_literal" => CodeTokenKind::Number,
        _ => inherited,
    };
    let function = matches!(
        node.kind(),
        "call" | "call_expression" | "method_invocation"
    )
    .then(|| {
        node.child_by_field_name("function")
            .or_else(|| node.child_by_field_name("name"))
    })
    .flatten();
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_code_leaves(
            child,
            if function.is_some_and(|function| function.id() == child.id()) {
                CodeTokenKind::Function
            } else {
                child_kind
            },
            source,
            leaves,
        );
    }
}

trait ParseCodeTokenKind {
    fn parse_kind(&self) -> Option<CodeTokenKind>;
}

impl ParseCodeTokenKind for str {
    fn parse_kind(&self) -> Option<CodeTokenKind> {
        match self {
            "line_comment" | "block_comment" | "comment" => Some(CodeTokenKind::Comment),
            "string_literal" | "raw_string_literal" | "char_literal" | "string"
            | "string_content" | "string_fragment" | "quoted_string" => Some(CodeTokenKind::String),
            "integer_literal"
            | "float_literal"
            | "number"
            | "number_literal"
            | "integer_expression"
            | "float_expression"
            | "integer"
            | "float"
            | "decimal_integer_literal"
            | "hex_integer_literal"
            | "binary_integer_literal"
            | "decimal_floating_point_literal"
            | "hex_floating_point_literal" => Some(CodeTokenKind::Number),
            "type_identifier" | "primitive_type" | "predefined_type" => Some(CodeTokenKind::Type),
            "field_identifier" | "property_identifier" => Some(CodeTokenKind::Property),
            "true" | "false" | "null" | "boolean_literal" | "none" | "None" => {
                Some(CodeTokenKind::Constant)
            }
            "as" | "async" | "await" | "break" | "case" | "catch" | "class" | "const"
            | "continue" | "crate" | "def" | "default" | "del" | "do" | "dyn" | "elif" | "else"
            | "enum" | "except" | "export" | "extends" | "extern" | "finally" | "for" | "from"
            | "func" | "fn" | "foreach" | "gen" | "global" | "goto" | "if" | "implements"
            | "import" | "in" | "instanceof" | "interface" | "is" | "lambda" | "let" | "loop"
            | "match" | "mod" | "module" | "move" | "mut" | "namespace" | "new" | "nonlocal"
            | "not" | "or" | "package" | "pass" | "private" | "protected" | "pub" | "public"
            | "raise" | "ref" | "return" | "self" | "Self" | "static" | "struct" | "super"
            | "switch" | "synchronized" | "this" | "throw" | "trait" | "try" | "type"
            | "typeof" | "union" | "unsafe" | "use" | "var" | "virtual" | "where" | "while"
            | "with" | "yield" => Some(CodeTokenKind::Keyword),
            _ => None,
        }
    }
}

fn classify_leaf(node: Node<'_>, source: &str) -> Option<CodeTokenKind> {
    let parent = node.parent()?;
    if node.kind() == "identifier" {
        if matches!(
            parent.kind(),
            "function_item"
                | "function_declaration"
                | "function_definition"
                | "method_declaration"
                | "method_definition"
                | "constructor_declaration"
        ) && parent
            .child_by_field_name("name")
            .is_some_and(|name| name.id() == node.id())
        {
            return Some(CodeTokenKind::Function);
        }
        if parent.kind() == "pair"
            && parent
                .child_by_field_name("key")
                .is_some_and(|key| key.id() == node.id())
        {
            return Some(CodeTokenKind::Type);
        }
    }
    let text = &source[node.start_byte()..node.end_byte()];
    if parent.kind() == "call_expression"
        && parent
            .child_by_field_name("function")
            .is_some_and(|function| {
                function.start_byte() <= node.start_byte() && function.end_byte() >= node.end_byte()
            })
        && !text.is_empty()
    {
        return Some(CodeTokenKind::Function);
    }
    node.kind().parse_kind()
}

fn push_code_token(tokens: &mut Vec<CodeToken>, range: Range<usize>, kind: CodeTokenKind) {
    if range.is_empty() {
        return;
    }
    if let Some(previous) = tokens.last_mut() {
        if previous.kind == kind && previous.range.end == range.start {
            previous.range.end = range.end;
            return;
        }
    }
    tokens.push(CodeToken { range, kind });
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
mod code_highlighting_tests {
    use super::{CodeTokenKind, PreparedPreview};
    use crate::rendered::RenderBlock;

    #[test]
    fn highlights_requested_programming_languages() {
        let examples = [
            (
                "nix",
                "# comment\nlet count = 42; in \"hello\"",
            ),
            (
                "javascript",
                "// comment\nfunction greet() { const count = 42; return \"hello\"; }",
            ),
            (
                "c",
                "// comment\nint main() { int count = 42; return \"hello\"; }",
            ),
            (
                "java",
                "// comment\nclass Example { String greet() { int count = 42; return \"hello\"; } }",
            ),
            (
                "python",
                "# comment\ndef greet():\n    count = 42\n    return \"hello\"",
            ),
            (
                "csharp",
                "// comment\nclass Example { string Greet() { int count = 42; return \"hello\"; } }",
            ),
        ];

        for (language, source) in examples {
            let blocks = [RenderBlock::Code {
                language: Some(language.to_owned()),
                code: source.to_owned(),
            }];
            let prepared = PreparedPreview::new(&blocks);
            let kinds = prepared
                .code_tokens(Some(language), source)
                .iter()
                .map(|token| token.kind)
                .collect::<Vec<_>>();

            assert!(kinds.contains(&CodeTokenKind::Keyword), "{language}");
            assert!(kinds.contains(&CodeTokenKind::String), "{language}");
            assert!(kinds.contains(&CodeTokenKind::Comment), "{language}");
            assert!(kinds.contains(&CodeTokenKind::Number), "{language}");
        }
    }
}

#[cfg(test)]
#[path = "tests/preview.rs"]
mod tests;
