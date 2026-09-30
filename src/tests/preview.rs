use super::*;

#[test]
fn prepares_inline_image_between_text_and_links() {
    let text = "See image:images/icon.png[Icon,24] and xref:note.adoc[note].";
    let prepared = PreparedPreview::new(&[RenderBlock::Paragraph(text.into())]);
    let parts = prepared.inline(text);
    assert_eq!(parts.len(), 5);
    assert_eq!(parts[1].text.as_ref(), "Icon");
    assert_eq!(
        parts[1].image.as_ref().unwrap().target.as_ref(),
        "images/icon.png"
    );
    assert_eq!(parts[1].image.as_ref().unwrap().width, Some(24));
    assert_eq!(parts[3].target.as_deref(), Some("note.adoc"));
}

#[test]
fn prepares_inline_links_and_nested_diagrams_once() {
    let text = "é [[note|界]] and xref:other.adoc[Other] end";
    let source = "a -> b\nb -> c";
    let prepared = PreparedPreview::new(&[
        RenderBlock::Paragraph(text.into()),
        RenderBlock::Admonition {
            kind: "NOTE".into(),
            blocks: vec![
                RenderBlock::Quote(text.into()),
                RenderBlock::Diagram {
                    kind: "mermaid".into(),
                    title: None,
                    source: source.into(),
                },
            ],
        },
        RenderBlock::Paragraph("".into()),
    ]);
    let parts = prepared.inline(text);
    assert_eq!(
        parts
            .iter()
            .map(|part| part.text.as_ref())
            .collect::<String>(),
        "é 界 and Other end"
    );
    assert_eq!(parts[1].target.as_deref(), Some("note.adoc"));
    assert_eq!(parts[3].target.as_deref(), Some("other.adoc"));
    assert_eq!(prepared.inline("")[0].text.as_ref(), "");
    assert_eq!(prepared.inline.len(), 2);
    for _ in 0..100 {
        assert!(std::ptr::eq(parts, prepared.inline(text)));
        assert_eq!(prepared.edges(source), diagram_edges(source));
    }
}

#[test]
fn scrolling_through_a_large_list_only_builds_a_bounded_chunk() {
    let items: Vec<_> = (0..10_000).map(|n| format!("item {n}")).collect();
    let blocks = vec![RenderBlock::OrderedList(items.clone())];
    let rows = PreviewRow::prepare(&blocks);
    assert!(rows.len() > 600);
    for index in [0, 1, 300, rows.len() - 1, 300, 1, 0] {
        let row = &rows[index];
        let RenderBlock::OrderedList(items) = &blocks[row.block_index] else {
            panic!()
        };
        let chunk = row.items(items);
        assert!(chunk.len() <= PREVIEW_CHUNK_SIZE);
        assert_eq!(chunk, &items[row.range.clone()]);
        assert!(std::ptr::eq(
            chunk.as_ptr(),
            items[row.range.clone()].as_ptr()
        ));
        assert_eq!(row.range.start, index * PREVIEW_CHUNK_SIZE);
    }
    assert!(rows[0].first);
    assert!(rows.last().unwrap().last);
}

#[test]
fn code_chunks_preserve_unicode_blank_lines_and_trailing_newlines() {
    for ending in ["", "\n"] {
        let code = format!("{}last{ending}", "界\n\n".repeat(1000));
        let blocks = vec![RenderBlock::Code {
            language: Some("rust".into()),
            code: code.clone(),
        }];
        let rows = PreviewRow::prepare(&blocks);
        assert!(rows.len() > 100);
        let mut restored = String::new();
        for row in rows {
            let RenderBlock::Code { language, code } = &blocks[row.block_index] else {
                panic!()
            };
            let code = row.text(code);
            assert_eq!(language.as_deref(), Some("rust"));
            assert!(code.lines().count() <= PREVIEW_CHUNK_SIZE);
            restored.push_str(code);
        }
        assert_eq!(restored, code);
    }
}

#[test]
fn table_rows_are_neither_lost_nor_duplicated_at_chunk_boundaries() {
    let cells: Vec<_> = (0..513)
        .map(|n| vec![n.to_string(), "cell".into()])
        .collect();
    let blocks = vec![RenderBlock::Table(cells.clone())];
    let rows = PreviewRow::prepare(&blocks);
    assert!(rows.len() > 1);
    let mut restored = Vec::new();
    for row in rows {
        let RenderBlock::Table(items) = &blocks[row.block_index] else {
            panic!()
        };
        let chunk = row.items(items);
        assert!(chunk.len() <= PREVIEW_CHUNK_SIZE);
        restored.extend(chunk.iter().cloned());
    }
    assert_eq!(restored, cells);
}
