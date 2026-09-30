use super::*;

pub(super) fn render(block: &RenderBlock, cx: &RenderContext<'_>) -> AnyElement {
    let RenderBlock::Diagram {
        kind,
        title,
        source,
    } = block
    else {
        unreachable!("renderer and block variant diverged")
    };
    let index = cx.index;
    let prepared = cx.prepared;
    let text_color = cx.text_color;
    let muted_text = cx.muted_text;
    let accent = cx.accent;
    let border = cx.border;
    let panel = cx.panel;
    {
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
}
