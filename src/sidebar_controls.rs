use super::*;

impl super::Emerald {
    pub(super) fn sidebar_toggle_button(
        &self,
        links: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let theme = self.theme();
        let visible = if links {
            self.interaction.links_sidebar_visible
        } else {
            self.interaction.sidebar_visible
        };
        let icon = themed_sidebar_icon(theme);
        let glow: gpui::Hsla = rgb(theme.accent).into();
        let button = div()
            .id(if links {
                "toggle-links-sidebar"
            } else {
                "toggle-sidebar"
            })
            .role(gpui::Role::Button)
            .aria_label(if links {
                "Toggle linked notes sidebar"
            } else {
                "Toggle notes sidebar"
            })
            .tab_stop(true)
            .cursor_pointer()
            .p_1()
            .rounded_sm()
            .hover(|b| b.bg(rgb(theme.hover)))
            .when(visible, |b| {
                b.shadow(vec![gpui::BoxShadow::new(
                    px(0.0),
                    px(0.0),
                    glow.opacity(0.7),
                )
                .blur_radius(px(10.0))])
            })
            .child(gpui::svg().data(icon.as_bytes()).size(px(20.0)));
        if links {
            button.on_click(cx.listener(Self::toggle_links_sidebar))
        } else {
            button.on_click(cx.listener(Self::toggle_sidebar))
        }
    }

    pub(super) fn toggle_sidebar(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.interaction.sidebar_visible = !self.interaction.sidebar_visible;
        cx.notify();
    }

    pub(super) fn toggle_links_sidebar(
        &mut self,
        _: &gpui::ClickEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.interaction.links_sidebar_visible = !self.interaction.links_sidebar_visible;
        cx.notify();
    }
}

pub(super) fn themed_sidebar_icon(theme: emerald::theme::EmeraldTheme) -> String {
    include_str!("../assets/bar-left-svgrepo-com.svg")
        .replace("#000000", &format!("#{:06x}", theme.accent))
}
