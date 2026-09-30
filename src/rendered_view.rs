use super::*;
use emerald::preview::PreparedPreview;
use std::path::Path;

impl super::Emerald {
    pub(super) fn render_preview_row(
        index: usize,
        row: &emerald::preview::PreviewRow,
        blocks: &[RenderBlock],
        style: RenderStyle,
        prepared: &PreparedPreview,
        document_path: &Path,
        entity: gpui::Entity<Emerald>,
    ) -> AnyElement {
        Self::render_block_contents(
            index,
            &blocks[row.block_index],
            style,
            prepared,
            document_path,
            entity,
            row,
        )
    }

    pub(super) fn render_block(
        index: usize,
        block: &RenderBlock,
        style: RenderStyle,
        prepared: &PreparedPreview,
        document_path: &Path,
        entity: gpui::Entity<Emerald>,
    ) -> AnyElement {
        Self::render_block_contents(
            index,
            block,
            style,
            prepared,
            document_path,
            entity,
            &emerald::preview::PreviewRow {
                block_index: index,
                range: 0..0,
                first: true,
                last: true,
            },
        )
    }

    fn render_block_contents(
        index: usize,
        block: &RenderBlock,
        style: RenderStyle,
        prepared: &PreparedPreview,
        document_path: &Path,
        entity: gpui::Entity<Emerald>,
        slice: &emerald::preview::PreviewRow,
    ) -> AnyElement {
        let theme = EVERFOREST_DARK;
        let text_color = style.text.unwrap_or(theme.text);
        let muted_text = style.muted_text.unwrap_or(theme.muted_text);
        let accent = style.accent.unwrap_or(theme.accent);
        let border = style.border.unwrap_or(theme.panel_border);
        let panel = style.panel.unwrap_or(theme.sidebar);
        let heading = style.heading.unwrap_or(text_color);
        let code_background = style.code_background.unwrap_or(theme.sidebar);
        let table_background = style.table_background.unwrap_or(theme.panel);
        let quote = style.quote.unwrap_or(muted_text);

        match block {
            RenderBlock::TableOfContents(entries) => div()
                .id(("table-of-contents", index))
                .flex()
                .flex_col()
                .gap_1()
                .w_full()
                .min_w(px(0.0))
                .px_3()
                .border_x_1()
                .when(slice.first, |el| el.pt_3().border_t_1().rounded_t_md())
                .when(slice.last, |el| el.pb_3().border_b_1().rounded_b_md())
                .when(!slice.last, |el| el.pb_1())
                .border_color(rgb(border))
                .bg(rgb(panel))
                .when(slice.first, |el| {
                    el.child(div().text_xs().text_color(rgb(accent)).child("Contents"))
                })
                .children(slice.items(entries).iter().map(|entry| {
                    div()
                        .min_w(px(0.0))
                        .whitespace_normal()
                        .pl(px(((entry.level.saturating_sub(2)) * 16) as f32))
                        .text_color(rgb(text_color))
                        .children(Self::render_text_parts(
                            &entry.text,
                            prepared,
                            document_path,
                            entity.clone(),
                        ))
                }))
                .into_any_element(),
            RenderBlock::Heading { level, text } => div()
                .flex()
                .flex_wrap()
                .w_full()
                .min_w(px(0.0))
                .whitespace_normal()
                .text_color(rgb(heading))
                .text_size(px(match level {
                    1 => 30.0,
                    2 => 24.0,
                    3 => 20.0,
                    _ => 18.0,
                }))
                .children(Self::render_text_parts(
                    text,
                    prepared,
                    document_path,
                    entity.clone(),
                ))
                .into_any_element(),
            RenderBlock::Paragraph(text) => div()
                .flex()
                .flex_wrap()
                .w_full()
                .min_w(px(0.0))
                .whitespace_normal()
                .text_color(rgb(text_color))
                .text_lg()
                .line_height(px(SOURCE_LINE_HEIGHT))
                .children(Self::render_text_parts(
                    text,
                    prepared,
                    document_path,
                    entity.clone(),
                ))
                .into_any_element(),
            RenderBlock::Image { target, alt, width } => {
                let path = document_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(target);
                let label = format!("Image unavailable: {alt} ({target})");
                let image = gpui::img(path)
                    .w_full()
                    .max_w(px(width.unwrap_or(760) as f32))
                    .with_fallback(move || {
                        div()
                            .text_color(rgb(muted_text))
                            .child(label.clone())
                            .into_any_element()
                    });
                div()
                    .w_full()
                    .min_w(px(0.0))
                    .child(image)
                    .into_any_element()
            }
            RenderBlock::UnorderedList(items) => div()
                .flex()
                .flex_col()
                .gap_2()
                .when(!slice.last, |el| el.pb_2())
                .w_full()
                .min_w(px(0.0))
                .children(slice.items(items).iter().map(|item| {
                    div()
                        .flex()
                        .w_full()
                        .min_w(px(0.0))
                        .gap_2()
                        .text_color(rgb(text_color))
                        .child(div().text_color(rgb(accent)).child("-"))
                        .child(
                            div()
                                .flex()
                                .flex_1()
                                .min_w(px(0.0))
                                .whitespace_normal()
                                .flex_wrap()
                                .children(Self::render_text_parts(
                                    item,
                                    prepared,
                                    document_path,
                                    entity.clone(),
                                )),
                        )
                }))
                .into_any_element(),
            RenderBlock::OrderedList(items) => div()
                .flex()
                .flex_col()
                .gap_2()
                .when(!slice.last, |el| el.pb_2())
                .w_full()
                .min_w(px(0.0))
                .children(
                    slice
                        .items(items)
                        .iter()
                        .enumerate()
                        .map(|(item_index, item)| {
                            div()
                                .flex()
                                .w_full()
                                .min_w(px(0.0))
                                .gap_2()
                                .text_color(rgb(text_color))
                                .child(
                                    div()
                                        .text_color(rgb(accent))
                                        .child(format!("{}.", slice.range.start + item_index + 1)),
                                )
                                .child(
                                    div()
                                        .flex()
                                        .flex_1()
                                        .min_w(px(0.0))
                                        .whitespace_normal()
                                        .flex_wrap()
                                        .children(Self::render_text_parts(
                                            item,
                                            prepared,
                                            document_path,
                                            entity.clone(),
                                        )),
                                )
                        }),
                )
                .into_any_element(),
            RenderBlock::Quote(text) => div()
                .flex()
                .flex_wrap()
                .whitespace_normal()
                .w_full()
                .min_w(px(0.0))
                .pl_3()
                .py_1()
                .border_l_4()
                .border_color(rgb(accent))
                .text_color(rgb(quote))
                .line_height(px(SOURCE_LINE_HEIGHT))
                .children(Self::render_text_parts(
                    text,
                    prepared,
                    document_path,
                    entity.clone(),
                ))
                .into_any_element(),
            RenderBlock::ThematicBreak => div()
                .w_full()
                .h(px(1.0))
                .flex_shrink_0()
                .bg(rgb(border))
                .into_any_element(),
            RenderBlock::Admonition { kind, blocks } => {
                let (kind_accent, icon) = admonition_colors(kind, accent);
                div()
                    .flex()
                    .flex_col()
                    .gap_3()
                    .w_full()
                    .min_w(px(0.0))
                    .p_3()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(kind_accent))
                    .border_l_4()
                    .bg(rgb(panel))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap_2()
                            .text_xs()
                            .text_color(rgb(kind_accent))
                            .child(icon)
                            .child(kind.clone()),
                    )
                    .children(blocks.iter().enumerate().map(|(child_index, block)| {
                        Self::render_block(
                            child_index,
                            block,
                            style,
                            prepared,
                            document_path,
                            entity.clone(),
                        )
                    }))
                    .into_any_element()
            }
            RenderBlock::Code { language, code } => div()
                .id(("code-block", index))
                .flex()
                .flex_col()
                .gap_2()
                .w_full()
                .min_w(px(0.0))
                .px_3()
                .bg(rgb(code_background))
                .border_x_1()
                .when(slice.first, |el| el.pt_3().border_t_1().rounded_t_md())
                .when(slice.last, |el| el.pb_3().border_b_1().rounded_b_md())
                .border_color(rgb(border))
                .when(slice.first, |el| {
                    el.child(
                        div()
                            .text_xs()
                            .text_color(rgb(accent))
                            .child(language.clone().unwrap_or_else(|| "text".to_string())),
                    )
                })
                .child(
                    div()
                        .min_w(px(0.0))
                        .whitespace_normal()
                        .text_color(rgb(text_color))
                        .line_height(px(SOURCE_LINE_HEIGHT - 4.0))
                        .children(
                            slice
                                .text(code)
                                .lines()
                                .map(|line| div().child(line.to_string())),
                        ),
                )
                .into_any_element(),
            RenderBlock::Diagram {
                kind,
                title,
                source,
            } => {
                let edges = prepared.edges(source);
                div()
                    .id(("diagram-block", index))
                    .flex()
                    .flex_col()
                    .gap_2()
                    .w_full()
                    .min_w(px(0.0))
                    .p_3()
                    .bg(rgb(panel))
                    .border_1()
                    .border_color(rgb(accent))
                    .rounded_md()
                    .child(
                        div()
                            .flex()
                            .gap_2()
                            .text_xs()
                            .text_color(rgb(accent))
                            .child(kind.clone())
                            .when_some(title.clone(), |header, title| header.child(title)),
                    )
                    .children(edges.iter().map(|edge| {
                        div()
                            .flex()
                            .flex_wrap()
                            .items_center()
                            .gap_2()
                            .min_w(px(0.0))
                            .text_color(rgb(text_color))
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .border_1()
                                    .border_color(rgb(border))
                                    .rounded_md()
                                    .child(edge.from.clone()),
                            )
                            .child(div().text_color(rgb(accent)).child("->"))
                            .child(
                                div()
                                    .px_2()
                                    .py_1()
                                    .border_1()
                                    .border_color(rgb(border))
                                    .rounded_md()
                                    .child(edge.to.clone()),
                            )
                    }))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .gap_1()
                            .min_w(px(0.0))
                            .whitespace_normal()
                            .text_color(rgb(muted_text))
                            .line_height(px(22.0))
                            .children(source.lines().map(|line| div().child(line.to_string()))),
                    )
                    .into_any_element()
            }
            RenderBlock::Table(rows) => div()
                .flex()
                .flex_col()
                .w_full()
                .min_w(px(0.0))
                .border_x_1()
                .when(slice.first, |el| el.border_t_1().rounded_t_md())
                .when(slice.last, |el| el.border_b_1().rounded_b_md())
                .border_color(rgb(border))
                .bg(rgb(table_background))
                .children(slice.items(rows).iter().map(|row| {
                    div()
                        .flex()
                        .flex_wrap()
                        .w_full()
                        .min_w(px(0.0))
                        .border_b_1()
                        .border_color(rgb(border))
                        .children(row.iter().map(|cell| {
                            div()
                                .flex_1()
                                .min_w(px(160.0))
                                .p_2()
                                .border_r_1()
                                .border_color(rgb(border))
                                .whitespace_normal()
                                .text_color(rgb(text_color))
                                .children(Self::render_text_parts(
                                    cell,
                                    prepared,
                                    document_path,
                                    entity.clone(),
                                ))
                        }))
                }))
                .into_any_element(),
        }
    }
}

/// Returns an Everforest-friendly accent and the label icon for each AsciiDoc
/// admonition kind. The fallback keeps custom themes' accent colour intact.
fn admonition_colors(kind: &str, fallback: u32) -> (u32, &'static str) {
    match kind.to_ascii_uppercase().as_str() {
        "NOTE" => (0x7fbbb3, "ⓘ"),
        "TIP" => (0x8fbf9f, "✦"),
        "IMPORTANT" => (0xd699b6, "◆"),
        "CAUTION" => (0xdbbc7f, "▲"),
        "WARNING" => (0xe67e80, "⚠"),
        _ => (fallback, "●"),
    }
}

#[cfg(test)]
mod tests {
    use super::admonition_colors;

    #[test]
    fn admonition_kinds_have_distinct_theme_accents() {
        let kinds = ["NOTE", "TIP", "IMPORTANT", "CAUTION", "WARNING"];
        let colors = kinds
            .iter()
            .map(|kind| admonition_colors(kind, 0).0)
            .collect::<Vec<_>>();
        assert_eq!(
            colors.windows(2).filter(|pair| pair[0] != pair[1]).count(),
            4
        );
        assert_eq!(admonition_colors("custom", 0x123456).0, 0x123456);
    }
}
