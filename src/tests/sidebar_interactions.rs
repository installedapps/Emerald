use super::*;
use gpui::{point, px, MouseDownEvent, MouseMoveEvent, TestAppContext};

#[gpui::test]
fn both_sidebar_toggle_handlers_change_only_their_panel(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    let state = EditorState::open_or_create(temp.path()).unwrap();
    let cx = cx.add_empty_window();
    let view = cx.new(|cx| Emerald::with_state(state, cx));
    let window = cx.windows()[0];
    cx.update_window(window, |_, window, app| {
        view.update(app, |view, cx| {
            view.toggle_sidebar(&gpui::ClickEvent::default(), window, cx)
        });
        view.update(app, |view, cx| {
            view.toggle_links_sidebar(&gpui::ClickEvent::default(), window, cx)
        });
    })
    .unwrap();
    view.read_with(cx, |view, _| {
        assert!(!view.interaction.sidebar_visible);
        assert!(!view.interaction.links_sidebar_visible);
    });
}

#[gpui::test]
fn opening_a_linked_sidebar_target_switches_to_the_note(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("a.adoc"), "xref:b.adoc[Open B]").unwrap();
    std::fs::write(temp.path().join("b.adoc"), "Target note").unwrap();
    let state = EditorState::open_or_create(temp.path()).unwrap();
    let cx = cx.add_empty_window();
    let view = cx.new(|cx| Emerald::with_state(state, cx));
    let window = cx.windows()[0];
    cx.update_window(window, |_, window, app| {
        view.update(app, |view, cx| view.open_link("b.adoc".into(), window, cx));
    })
    .unwrap();
    cx.run_until_parked();
    assert!(view.read_with(cx, |view, _| view.state.active_file().ends_with("b.adoc")));
}

#[gpui::test]
fn dragging_both_resize_edges_updates_width_and_release_clears_drag_state(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    let state = EditorState::open_or_create(temp.path()).unwrap();
    let cx = cx.add_empty_window();
    let view = cx.new(|cx| Emerald::with_state(state, cx));
    let window = cx.windows()[0];
    cx.update_window(window, |_, window, app| {
        view.update(app, |view, cx| {
            view.begin_sidebar_resize(
                false,
                &MouseDownEvent {
                    position: point(px(300.0), px(4.0)),
                    ..Default::default()
                },
                window,
                cx,
            );
            view.resize_sidebar(
                &MouseMoveEvent {
                    position: point(px(340.0), px(4.0)),
                    ..Default::default()
                },
                window,
                cx,
            );
            assert_eq!(view.interaction.sidebar_width, 320.0);
            view.finish_sidebar_resize(&Default::default(), window, cx);
            view.begin_sidebar_resize(
                true,
                &MouseDownEvent {
                    position: point(px(500.0), px(4.0)),
                    ..Default::default()
                },
                window,
                cx,
            );
            view.resize_sidebar(
                &MouseMoveEvent {
                    position: point(px(450.0), px(4.0)),
                    ..Default::default()
                },
                window,
                cx,
            );
            assert_eq!(view.interaction.links_sidebar_width, 310.0);
            view.finish_sidebar_resize(&Default::default(), window, cx);
            assert!(view.interaction.sidebar_resize.is_none());
            assert!(view.interaction.links_sidebar_resize.is_none());
        });
    })
    .unwrap();
}
