use super::*;

#[test]
fn renders_block_image_macros_with_alt_text_and_width() {
    assert_eq!(
        render_blocks("Before.\n\nimage::Test.jpg[Photo,640]\n\nAfter."),
        vec![
            RenderBlock::Paragraph("Before.".into()),
            RenderBlock::Image {
                target: "Test.jpg".into(),
                alt: "Photo".into(),
                width: Some(640),
            },
            RenderBlock::Paragraph("After.".into()),
        ]
    );
    assert_eq!(
        render_blocks("An inline image: image:icon.png[Icon,24]."),
        vec![RenderBlock::Paragraph(
            "An inline image: image:icon.png[Icon,24].".into()
        )]
    );
}

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
        render_blocks("[NOTE]\n====\nFirst paragraph.\n\nSecond paragraph.\n\n* One\n* Two\n===="),
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

#[test]
fn preview_pipeline_accepts_alternate_stages_and_toc_policy() {
    struct FixtureParser;

    impl DocumentParser for FixtureParser {
        fn parse(&self, _: &str) -> ParsedDocument {
            ParsedDocument {
                report: ParseReport {
                    parsed: true,
                    diagnostic_count: 2,
                },
                blocks: vec![ParsedBlock::Heading {
                    level: 1,
                    text: "Injected heading".into(),
                }],
            }
        }
    }

    let pipeline = PreviewPipeline::new(
        Arc::new(FixtureParser),
        Arc::new(AsciiDocBlockRenderer),
        Arc::new(DefaultPreviewPreparer),
    );
    let parsed = pipeline.parse(":toc:\n");
    let snapshot = pipeline.snapshot(":toc:\n", &parsed);
    assert!(snapshot.report.parsed);
    assert_eq!(snapshot.report.diagnostic_count, 2);
    assert!(matches!(
        snapshot.blocks.first(),
        Some(RenderBlock::TableOfContents(entries)) if entries[0].text == "Injected heading"
    ));

    let no_toc = pipeline.snapshot_with_options(
        ":toc:\n",
        &parsed,
        &RenderOptions {
            toc: TocPolicy::Disabled,
        },
    );
    assert!(matches!(
        no_toc.blocks.as_slice(),
        [RenderBlock::Heading { .. }]
    ));

    let mut cache = RenderCache::with_pipeline(pipeline);
    let cached = cache
        .snapshot_for_revision(":toc:\n", Path::new("custom.adoc"), 1, Some(&parsed))
        .unwrap();
    assert!(matches!(
        cached.blocks.first(),
        Some(RenderBlock::TableOfContents(entries)) if entries[0].text == "Injected heading"
    ));
}
