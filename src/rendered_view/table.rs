use super::*;

pub(super) fn render(block: &RenderBlock, cx: &RenderContext<'_>) -> AnyElement {
    let RenderBlock::Table(rows) = block else {
        unreachable!("renderer and block variant diverged")
    };
    let theme = cx.theme;
    let prepared = cx.prepared;
    let document_path = cx.document_path;
    let entity = cx.entity.clone();
    let slice = cx.slice;
    let text_color = cx.text_color;
    let border = cx.border;
    let table_background = cx.table_background;
    div()
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
                        .children(Emerald::render_text_parts(
                            cell,
                            prepared,
                            document_path,
                            theme,
                            entity.clone(),
                        ))
                }))
        }))
        .into_any_element()
}
