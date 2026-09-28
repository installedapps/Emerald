mod app;

use app::Emerald;
use gpui::{prelude::*, px, size, Application, Bounds, Focusable, WindowBounds, WindowOptions};

fn main() {
    emerald::logging::init();
    tracing::info!("starting Emerald");
    Application::new().run(|cx| {
        let window = cx
            .open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
                        None,
                        size(px(1200.0), px(760.0)),
                        cx,
                    ))),
                    titlebar: None,
                    ..Default::default()
                },
                |_, cx| cx.new(Emerald::new),
            )
            .unwrap_or_else(|error| {
                tracing::error!(error = %error, "failed to open Emerald window");
                panic!("could not open the Emerald window: {error}");
            });

        window
            .update(cx, |view, window, cx| {
                window.focus(&view.focus_handle(cx));
                cx.activate(true);
            })
            .unwrap_or_else(|error| {
                tracing::error!(error = %error, "failed to focus Emerald editor");
                panic!("could not focus the Emerald editor: {error}");
            });

        let view = window
            .update(cx, |_, _, cx| cx.entity())
            .unwrap_or_else(|error| {
                tracing::error!(error = %error, "failed to access Emerald view");
                panic!("could not access the Emerald view: {error}");
            });
        cx.intercept_keystrokes(move |event, _, cx| {
            view.update(cx, |view, cx| view.handle_keystroke(&event.keystroke, cx))
        })
        .detach();

        window
            .update(cx, |view, window, cx| {
                let blink_interval = view.blink_cursor.interval();
                cx.spawn_in(window, async move |view, cx| loop {
                    cx.background_executor().timer(blink_interval).await;
                    if view
                        .update(cx, |view, cx| {
                            view.blink_cursor.tick();
                            cx.notify();
                        })
                        .is_err()
                    {
                        break;
                    }
                })
                .detach();
            })
            .unwrap_or_else(|error| {
                tracing::error!(error = %error, "failed to start Emerald cursor blink");
                panic!("could not start the Emerald cursor blink: {error}");
            });
    });
}
