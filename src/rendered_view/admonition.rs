use super::*;

pub(super) fn render(block: &RenderBlock, cx: &RenderContext<'_>) -> AnyElement {
    let RenderBlock::Admonition { kind, blocks } = block else {
        unreachable!("renderer and block variant diverged")
    };
    let style = cx.style;
    let theme = cx.theme;
    let prepared = cx.prepared;
    let document_path = cx.document_path;
    let accent = cx.accent;
    let panel = cx.panel;
    let entity = cx.entity.clone();
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
            Emerald::render_block(
                child_index,
                block,
                style,
                theme,
                prepared,
                document_path,
                entity.clone(),
            )
        }))
        .into_any_element()
}

/// Returns an Everforest-friendly accent and the label icon for each AsciiDoc
/// admonition kind. The fallback keeps custom themes' accent colour intact.
pub(super) fn admonition_colors(kind: &str, fallback: u32) -> (u32, &'static str) {
    match kind.to_ascii_uppercase().as_str() {
        "NOTE" => (0x7fbbb3, "ⓘ"),
        "TIP" => (0x8fbf9f, "✦"),
        "IMPORTANT" => (0xd699b6, "◆"),
        "CAUTION" => (0xdbbc7f, "▲"),
        "WARNING" => (0xe67e80, "⚠"),
        _ => (fallback, "●"),
    }
}
