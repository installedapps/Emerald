use super::*;
use gpui::{point, px, size, MouseDownEvent, MouseMoveEvent, TestAppContext, VisualTestContext};

fn draw_sidebar(cx: &mut VisualTestContext, view: &gpui::Entity<Emerald>) {
    cx.draw(
        point(px(0.0), px(0.0)),
        size(px(900.0), px(700.0)),
        |_, _| view.clone().into_any_element(),
    );
}

fn invoke_action(cx: &mut VisualTestContext, view: &gpui::Entity<Emerald>, action: &str) {
    let window = cx.windows()[0];
    cx.update_window(window, |_, window, app| {
        view.update(app, |view, cx| match action {
            "new-file" => view.create_new_file(&gpui::ClickEvent::default(), window, cx),
            "save" => view.save_current_file(&gpui::ClickEvent::default(), window, cx),
            "open" => view.open_file(&gpui::ClickEvent::default(), window, cx),
            "delete" => view.delete_current_file(&gpui::ClickEvent::default(), window, cx),
            "graph" => view.toggle_graph(&gpui::ClickEvent::default(), window, cx),
            _ => panic!("unknown sidebar action {action}"),
        })
    })
    .unwrap();
}

#[gpui::test]
fn sidebar_action_rail_is_vertical_and_before_the_file_list(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    let state = EditorState::open_or_create(temp.path()).unwrap();
    let cx = cx.add_empty_window();
    let view = cx.new(|cx| Emerald::with_state(state, cx));

    draw_sidebar(cx, &view);

    let rail = cx.debug_bounds("sidebar-action-rail").unwrap();
    let files = cx.debug_bounds("sidebar-file-column").unwrap();
    assert!(rail.origin.x + rail.size.width <= files.origin.x);

    let actions = ["new-file", "save", "open", "delete", "graph"];
    let bounds: Vec<_> = actions
        .iter()
        .map(|selector| cx.debug_bounds(selector).unwrap())
        .collect();
    assert!(bounds
        .iter()
        .all(|bounds| bounds.size.width > px(0.0) && bounds.size.height > px(0.0)));
    assert!(bounds
        .windows(2)
        .all(|pair| { pair[0].origin.y + pair[0].size.height <= pair[1].origin.y }));
}

#[gpui::test]
fn sidebar_action_handlers_keep_their_functions(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    std::fs::write(temp.path().join("first.adoc"), "First note").unwrap();
    let second = temp.path().join("second.adoc");
    std::fs::write(&second, "Second note").unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.insert_text(" saved by button");
    let active_path = state.active_file().to_path_buf();
    let cx = cx.add_empty_window();
    let view = cx.new(|cx| Emerald::with_state(state, cx));

    invoke_action(cx, &view, "save");
    cx.run_until_parked();
    let saved = std::fs::read_to_string(&active_path).unwrap();
    assert!(
        saved.contains("saved by button"),
        "saved file was: {saved:?}"
    );

    invoke_action(cx, &view, "new-file");
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.state.files().len()), 4);

    invoke_action(cx, &view, "graph");
    assert!(view.read_with(cx, |view, _| view.interaction.graph_mode));

    invoke_action(cx, &view, "open");
    assert!(cx.did_prompt_for_paths());
    cx.simulate_path_prompt_response(|_| Some(vec![second.clone()]));
    cx.run_until_parked();
    assert!(view.read_with(cx, |view, _| view.state.active_file() == second));

    invoke_action(cx, &view, "delete");
    assert!(cx.has_pending_prompt());
    cx.simulate_prompt_answer("Delete");
    cx.run_until_parked();
    assert!(!second.exists());
}

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
