use super::*;

pub(super) fn render(block: &RenderBlock, cx: &RenderContext<'_>) -> AnyElement {
    let RenderBlock::ThematicBreak = block else {
        unreachable!("renderer and block variant diverged")
    };
    let border = cx.border;
    div()
        .w_full()
        .h(px(1.0))
        .flex_shrink_0()
        .bg(rgb(border))
        .into_any_element()
}
