use super::*;

pub(super) fn render(block: &RenderBlock, cx: &RenderContext<'_>) -> AnyElement {
    let RenderBlock::OrderedList(items) = block else {
        unreachable!("renderer and block variant diverged")
    };
    let theme = cx.theme;
    let prepared = cx.prepared;
    let document_path = cx.document_path;
    let entity = cx.entity.clone();
    let slice = cx.slice;
    let text_color = cx.text_color;
    let accent = cx.accent;
    div()
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
                                .children(Emerald::render_text_parts(
                                    item,
                                    prepared,
                                    document_path,
                                    theme,
                                    entity.clone(),
                                )),
                        )
                }),
        )
        .into_any_element()
}
