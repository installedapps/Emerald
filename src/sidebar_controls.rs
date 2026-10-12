use super::sidebar::{sidebar_icon, SidebarIcon};
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
        let glow: gpui::Hsla = rgb(theme.accent).into();
        let button = div()
            .id(if links {
                "toggle-links-sidebar"
            } else {
                "toggle-sidebar"
            })
            .role(gpui::Role::Button)
            .aria_label(if links && visible {
                "Hide linked notes sidebar"
            } else if links {
                "Show linked notes sidebar"
            } else if visible {
                "Hide notes sidebar"
            } else {
                "Show notes sidebar"
            })
            .tab_stop(true)
            .cursor_pointer()
            .focus_visible(|button| button.bg(rgb(theme.hover)).text_color(rgb(theme.text)))
            .px_2()
            .py_1()
            .rounded_md()
            .text_sm()
            .text_color(rgb(if visible {
                theme.accent
            } else {
                theme.muted_text
            }))
            .hover(|button| button.bg(rgb(theme.hover)).text_color(rgb(theme.text)))
            .when(visible, |b| {
                b.shadow(vec![gpui::BoxShadow::new(
                    px(0.0),
                    px(0.0),
                    glow.opacity(0.7),
                )
                .blur_radius(px(10.0))])
            })
            .child(sidebar_icon(
                SidebarIcon::NotesPanel { links, visible },
                theme,
                visible,
                16.0,
            ));
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
