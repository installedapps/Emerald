use super::*;

pub(super) fn render(block: &RenderBlock, cx: &RenderContext<'_>) -> AnyElement {
    let RenderBlock::Paragraph(text) = block else {
        unreachable!("renderer and block variant diverged")
    };
    let theme = cx.theme;
    let prepared = cx.prepared;
    let document_path = cx.document_path;
    let entity = cx.entity.clone();
    let text_color = cx.text_color;
    div()
        .flex()
        .flex_wrap()
        .w_full()
        .min_w(px(0.0))
        .whitespace_normal()
        .text_color(rgb(text_color))
        .text_lg()
        .line_height(px(SOURCE_LINE_HEIGHT))
        .children(Emerald::render_text_parts(
            text,
            prepared,
            document_path,
            theme,
            entity.clone(),
        ))
        .into_any_element()
}
