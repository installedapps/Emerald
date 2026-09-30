use super::*;

impl super::Emerald {
    pub(super) fn begin_sidebar_resize(
        &mut self,
        links: bool,
        event: &MouseDownEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let width = if links {
            self.interaction.links_sidebar_width
        } else {
            self.interaction.sidebar_width
        };
        let drag = (f32::from(event.position.x), width);
        if links {
            self.interaction.sidebar_resize = None;
            self.interaction.links_sidebar_resize = Some(drag);
        } else {
            self.interaction.links_sidebar_resize = None;
            self.interaction.sidebar_resize = Some(drag);
        }
        cx.notify();
    }

    pub(super) fn resize_sidebar(
        &mut self,
        event: &MouseMoveEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let x = f32::from(event.position.x);
        if let Some((start, width)) = self.interaction.sidebar_resize {
            self.interaction.sidebar_width = resized_width(width, start, x, false);
        }
        if let Some((start, width)) = self.interaction.links_sidebar_resize {
            self.interaction.links_sidebar_width = resized_width(width, start, x, true);
        }
        if self.interaction.sidebar_resize.is_some()
            || self.interaction.links_sidebar_resize.is_some()
        {
            cx.notify();
        }
    }

    pub(super) fn finish_sidebar_resize(
        &mut self,
        _: &gpui::MouseUpEvent,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let left = self.interaction.sidebar_resize.take().is_some();
        let right = self.interaction.links_sidebar_resize.take().is_some();
        if left || right {
            cx.notify();
        }
    }
}

pub(super) fn resized_width(width: f32, start_x: f32, current_x: f32, from_right: bool) -> f32 {
    let delta = current_x - start_x;
    (width + if from_right { -delta } else { delta }).clamp(180.0, 520.0)
}
