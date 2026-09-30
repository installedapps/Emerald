use super::*;

pub(super) fn render(block: &RenderBlock, cx: &RenderContext<'_>) -> AnyElement {
    let RenderBlock::Image { target, alt, width } = block else {
        unreachable!("renderer and block variant diverged")
    };
    let document_path = cx.document_path;
    let text_color = cx.text_color;
    let muted_text = cx.muted_text;
    {
        let path = document_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(target);
        let label = format!("Image unavailable: {alt} ({target})");
        let image = gpui::img(path)
            .w_full()
            .max_w(px(width.unwrap_or(760) as f32))
            .with_fallback(move || {
                div()
                    .text_color(rgb(muted_text))
                    .child(label.clone())
                    .into_any_element()
            });
        div()
            .w_full()
            .min_w(px(0.0))
            .child(image)
            .into_any_element()
    }
}
