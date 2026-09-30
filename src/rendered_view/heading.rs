use super::*;

pub(super) fn render(block: &RenderBlock, cx: &RenderContext<'_>) -> AnyElement {
    let RenderBlock::Heading { level, text } = block else {
        unreachable!("renderer and block variant diverged")
    };
    let theme = cx.theme;
    let prepared = cx.prepared;
    let document_path = cx.document_path;
    let entity = cx.entity.clone();
    let text_color = cx.text_color;
    let heading = cx.heading;
    div()
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
        .children(Emerald::render_text_parts(
            text,
            prepared,
            document_path,
            theme,
            entity.clone(),
        ))
        .into_any_element()
}
