use super::*;

pub(super) fn render(block: &RenderBlock, cx: &RenderContext<'_>) -> AnyElement {
    let RenderBlock::Quote(text) = block else {
        unreachable!("renderer and block variant diverged")
    };
    let theme = cx.theme;
    let prepared = cx.prepared;
    let document_path = cx.document_path;
    let entity = cx.entity.clone();
    let text_color = cx.text_color;
    let accent = cx.accent;
    let quote = cx.quote;
    div()
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
        .children(Emerald::render_text_parts(
            text,
            prepared,
            document_path,
            theme,
            entity.clone(),
        ))
        .into_any_element()
}
