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
    let mut diagram = div()
        .id(("diagram-block", index))
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .w_full()
        .min_w(px(0.0));

    if let Some(title) = title {
        diagram = diagram.child(
            div()
                .w_full()
                .text_sm()
                .text_color(rgb(cx.heading))
                .child(title.clone()),
        );
    }

    if kind == "mermaid" {
        if let Some(svg) = emerald::diagram::mermaid_svg(source) {
            let image = std::sync::Arc::new(gpui::Image::from_bytes(
                gpui::ImageFormat::Svg,
                svg.into_bytes(),
            ));
            return diagram
                .child(gpui::img(image).w_full().max_w(px(920.0)))
                .into_any_element();
        }
    }

    diagram
        .child(
            div()
                .w_full()
                .p_3()
                .rounded_md()
                .border_1()
                .border_color(rgb(cx.border))
                .text_color(rgb(cx.muted_text))
                .child("Unable to render this diagram"),
        )
        .into_any_element()
}
