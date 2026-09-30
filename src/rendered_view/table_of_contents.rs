use super::*;

pub(super) fn render(block: &RenderBlock, cx: &RenderContext<'_>) -> AnyElement {
    let RenderBlock::TableOfContents(entries) = block else {
        unreachable!("renderer and block variant diverged")
    };
    let index = cx.index;
    let theme = cx.theme;
    let prepared = cx.prepared;
    let document_path = cx.document_path;
    let entity = cx.entity.clone();
    let slice = cx.slice;
    let text_color = cx.text_color;
    let accent = cx.accent;
    let border = cx.border;
    let panel = cx.panel;
    div()
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
                .children(Emerald::render_text_parts(
                    &entry.text,
                    prepared,
                    document_path,
                    theme,
                    entity.clone(),
                ))
        }))
        .into_any_element()
}
