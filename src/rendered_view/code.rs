use super::*;

pub(super) fn render(block: &RenderBlock, cx: &RenderContext<'_>) -> AnyElement {
    let RenderBlock::Code { language, code } = block else {
        unreachable!("renderer and block variant diverged")
    };
    let index = cx.index;
    let slice = cx.slice;
    let text_color = cx.text_color;
    let accent = cx.accent;
    let border = cx.border;
    let code_background = cx.code_background;
    div()
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
        .into_any_element()
}
