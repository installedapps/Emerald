use super::*;
use gpui::AnimationExt;

impl super::Emerald {
    pub(super) fn sidebar_frame(
        &self,
        links: bool,
        content: impl IntoElement,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let (width, visible) = if links {
            (
                self.interaction.links_sidebar_width,
                self.interaction.links_sidebar_visible,
            )
        } else {
            (
                self.interaction.sidebar_width,
                self.interaction.sidebar_visible,
            )
        };
        let edge = self.sidebar_resize_edge(links, cx);
        div()
            .id(if links {
                "links-sidebar-slide"
            } else {
                "sidebar-slide"
            })
            .relative()
            .overflow_hidden()
            .min_w(px(0.0))
            .with_spring(
                if links {
                    "links-sidebar-width"
                } else {
                    "sidebar-width"
                },
                gpui::SpringAnimation::new(gpui::SpringConfig::new(900.0, 75.0, 1.0))
                    .to(if visible { width } else { 0.0 }),
                |element, animated_width| element.w(px(animated_width)),
            )
            .child(content)
            .when(visible, |panel| panel.child(edge))
    }

    fn sidebar_resize_edge(&self, links: bool, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = self.theme();
        let active = if links {
            self.interaction.links_sidebar_resize.is_some()
        } else {
            self.interaction.sidebar_resize.is_some()
        };
        let edge = div()
            .id(if links {
                "links-sidebar-resize-edge"
            } else {
                "sidebar-resize-edge"
            })
            .absolute()
            .top_0()
            .h_full()
            .w(px(5.0))
            .cursor_col_resize();
        let edge = if links { edge.left_0() } else { edge.right_0() };
        edge.when(active, |edge| {
            edge.bg(rgb(theme.accent)).shadow(vec![gpui::BoxShadow::new(
                px(0.0),
                px(0.0),
                rgb(theme.accent).into(),
            )
            .blur_radius(px(12.0))])
        })
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |view, event, window, cx| {
                view.begin_sidebar_resize(links, event, window, cx);
            }),
        )
    }
}
