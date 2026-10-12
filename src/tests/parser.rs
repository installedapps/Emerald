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

#[test]
fn parses_asciidoc_and_markdown_thematic_breaks() {
    for marker in ["'''", "---", "***", "- - -", "* * *"] {
        let document = ParsedDocument::from_source(&format!("Before.\n\n{marker}\n\nAfter."));

        assert!(document.report.parsed, "{marker}");
        assert_eq!(
            document.blocks,
            vec![
                ParsedBlock::Paragraph("Before.".into()),
                ParsedBlock::ThematicBreak,
                ParsedBlock::Paragraph("After.".into()),
            ],
            "{marker}"
        );
    }
}

#[test]
fn does_not_parse_spaced_apostrophes_or_long_markers_as_thematic_breaks() {
    let document = ParsedDocument::from_source("Before.\n\n' ' '\n\nAfter.");
    assert!(document.report.parsed);
    assert_eq!(
        document.blocks,
        vec![
            ParsedBlock::Paragraph("Before.".into()),
            ParsedBlock::Paragraph("' ' '".into()),
            ParsedBlock::Paragraph("After.".into()),
        ]
    );

    for marker in ["----", "****"] {
        let document = ParsedDocument::from_source(&format!("Before.\n\n{marker}\n\nAfter."));

        assert!(document.report.parsed, "{marker}");
        assert!(
            !document.blocks.contains(&ParsedBlock::ThematicBreak),
            "{marker}"
        );
    }
}

#[test]
fn thematic_break_requires_a_blank_line_after_a_paragraph() {
    let document = ParsedDocument::from_source("Before.\n'''\nAfter.");

    assert!(document.report.parsed);
    assert_eq!(
        document.blocks,
        vec![ParsedBlock::Paragraph("Before. ''' After.".into())]
    );
}

#[test]
fn fallback_parser_preserves_supported_thematic_breaks() {
    for marker in ["'''", "---", "***", "- - -", "* * *"] {
        assert_eq!(
            fallback_blocks(&format!("Before.\n\n{marker}\n\nAfter.")),
            vec![
                ParsedBlock::Paragraph("Before.".into()),
                ParsedBlock::ThematicBreak,
                ParsedBlock::Paragraph("After.".into()),
            ],
            "{marker}"
        );
    }
    assert_eq!(
        fallback_blocks("Before.\n\n' ' '\n\nAfter."),
        vec![
            ParsedBlock::Paragraph("Before.".into()),
            ParsedBlock::Paragraph("' ' '".into()),
            ParsedBlock::Paragraph("After.".into()),
        ]
    );
}
