use super::*;
use emerald::preview::PreviewRow;
use gpui::{point, ScrollDelta, ScrollWheelEvent, TestAppContext, VisualTestContext};

struct PreviewFixture {
    entity: gpui::Entity<Emerald>,
    blocks: Arc<Vec<RenderBlock>>,
    rows: Arc<Vec<PreviewRow>>,
    state: ListState,
    prepared: Arc<emerald::preview::PreparedPreview>,
}

struct DrawPreview(Option<AnyElement>);

impl Render for DrawPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        self.0.take().unwrap()
    }
}

impl PreviewFixture {
    fn draw(&self, cx: &mut VisualTestContext, width: f32) -> usize {
        let work = Rc::new(RefCell::new(0));
        let drawn = work.clone();
        let blocks = self.blocks.clone();
        let rows = self.rows.clone();
        let entity = self.entity.clone();
        let prepared = self.prepared.clone();
        let scrollbar_state = self.state.clone();
        cx.draw(point(px(0.), px(0.)), size(px(width), px(300.)), |_, cx| {
            let content = list(self.state.clone(), move |index, _, _| {
                let row = &rows[index];
                // Count the children actually handed to the production renderer,
                // not elapsed time, which is noisy on CI.
                *drawn.borrow_mut() += match &blocks[row.block_index] {
                    RenderBlock::OrderedList(items) | RenderBlock::UnorderedList(items) => {
                        row.items(items).len()
                    }
                    RenderBlock::Code { code, .. } => row.text(code).lines().count(),
                    RenderBlock::Table(items) => row.items(items).len(),
                    _ => 1,
                };
                Emerald::render_preview_row(
                    index,
                    row,
                    &blocks,
                    RenderStyle::default(),
                    emerald::theme::EVERFOREST_DARK,
                    &prepared,
                    std::path::Path::new("note.adoc"),
                    entity.clone(),
                )
            })
            .with_sizing_behavior(gpui::ListSizingBehavior::Auto)
            .w_full()
            .h_full();
            let element = div()
                .relative()
                .size_full()
                .child(content)
                .child(
                    div().absolute().inset_0().child(
                        gpui_base::Scrollbar::vertical(&scrollbar_state)
                            .mode(gpui_base::ScrollbarMode::Always)
                            .viewport_from_layout(),
                    ),
                )
                .into_any_element();
            cx.new(|_| DrawPreview(Some(element))).into_any_element()
        });
        let count = *work.borrow();
        count
    }
}

#[gpui::test]
fn document_scrollbar_track_click_moves_the_list(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    let state = EditorState::open_or_create(temp.path()).unwrap();
    let cx = cx.add_empty_window();
    let entity = cx.new(|cx| Emerald::with_state(state, cx));
    let blocks = Arc::new(vec![RenderBlock::UnorderedList(
        (0..1_000).map(|index| format!("item {index}")).collect(),
    )]);
    let rows = Arc::new(PreviewRow::prepare(&blocks));
    let fixture = PreviewFixture {
        state: ListState::new(rows.len(), ListAlignment::Top, px(300.)),
        prepared: Arc::new(emerald::preview::PreparedPreview::new(&blocks)),
        entity,
        blocks,
        rows,
    };

    fixture.draw(cx, 600.);
    let before = fixture.state.logical_scroll_top();
    cx.simulate_click(point(px(595.), px(260.)), gpui::Modifiers::default());
    fixture.draw(cx, 600.);

    assert!(fixture.state.logical_scroll_top().item_ix > before.item_ix);
}

fn scroll(cx: &mut VisualTestContext, delta: f32) {
    cx.simulate_event(ScrollWheelEvent {
        position: point(px(100.), px(100.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(delta))),
        ..Default::default()
    });
}

fn draw_source(
    cx: &mut VisualTestContext,
    entity: &gpui::Entity<Emerald>,
    origin: (f32, f32),
    viewport: (f32, f32),
) {
    cx.draw(
        point(px(origin.0), px(origin.1)),
        size(px(viewport.0), px(viewport.1)),
        |window, cx| {
            let element = entity.update(cx, |view, cx| view.render_source(cx.entity(), window));
            cx.new(|_| DrawPreview(Some(element))).into_any_element()
        },
    );
}

fn request_source_reveal(entity: &gpui::Entity<Emerald>, cx: &mut VisualTestContext) {
    entity.update(cx, |view, _| {
        view.interaction.source_cursor_reveal_pending = true;
        view.interaction.source_cursor_reveal_waiting_for_layout = false;
    });
}

fn settle_source_reveal(
    cx: &mut VisualTestContext,
    entity: &gpui::Entity<Emerald>,
    origin: (f32, f32),
    viewport: (f32, f32),
) {
    for _ in 0..12 {
        draw_source(cx, entity, origin, viewport);
    }
    entity.read_with(cx, |view, _| {
        let top = view.source_list_state.logical_scroll_top();
        assert!(
            !view.interaction.source_cursor_reveal_pending,
            "reveal did not settle: cursor={}, waiting={}, top={}:{}, rows={:?}",
            view.state.cursor(),
            view.interaction.source_cursor_reveal_waiting_for_layout,
            top.item_ix,
            f32::from(top.offset_in_item),
            view.interaction
                .source_geometry
                .borrow()
                .iter()
                .map(|row| (row.start, row.origin.1))
                .collect::<Vec<_>>()
        );
    });
}

fn assert_source_caret_visible(
    entity: &gpui::Entity<Emerald>,
    cx: &mut VisualTestContext,
    viewport_top: f32,
    viewport_height: f32,
) {
    entity.read_with(cx, |view, _| {
        let cursor = view.state.cursor();
        let geometry = view.interaction.source_geometry.borrow();
        let row = geometry
            .iter()
            .filter(|row| {
                let local = cursor.saturating_sub(row.start);
                cursor >= row.start && row.stops.last().is_some_and(|(end, _)| local <= *end)
            })
            .max_by_key(|row| row.start)
            .unwrap_or_else(|| {
                panic!(
                    "caret row at {cursor} should be painted; visible starts: {:?}",
                    geometry.iter().map(|row| row.start).collect::<Vec<_>>()
                )
            });
        assert!(
            row.origin.1 >= viewport_top,
            "caret row starts above viewport: {} < {viewport_top}",
            row.origin.1
        );
        assert!(
            row.origin.1 + row.height <= viewport_top + viewport_height,
            "caret row ends below viewport: {} > {}",
            row.origin.1 + row.height,
            viewport_top + viewport_height
        );
    });
}

#[gpui::test]
fn large_preview_scrolls_down_and_up_without_building_the_whole_block(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    let state = EditorState::open_or_create(temp.path()).unwrap();
    let cx = cx.add_empty_window();
    let entity = cx.new(|cx| Emerald::with_state(state, cx));
    for count in [1000, 10_000] {
        for block in [
            RenderBlock::OrderedList((0..count).map(|n| format!("item {n}")).collect()),
            RenderBlock::Code {
                language: None,
                code: "line\n".repeat(count),
            },
            RenderBlock::Table(
                (0..count)
                    .map(|n| vec![n.to_string(), "cell".into()])
                    .collect(),
            ),
        ] {
            let blocks = Arc::new(vec![block]);
            let rows = Arc::new(PreviewRow::prepare(&blocks));
            let prepared = Arc::new(emerald::preview::PreparedPreview::new(&blocks));
            if count == 1000 {
                // Reproduce the previous one-list-item-per-block strategy with
                // the same renderer and viewport before checking the new one.
                let baseline = PreviewFixture {
                    state: ListState::new(1, ListAlignment::Top, px(300.)),
                    prepared: prepared.clone(),
                    entity: entity.clone(),
                    blocks: blocks.clone(),
                    rows: Arc::new(vec![PreviewRow {
                        block_index: 0,
                        range: 0..0,
                        first: true,
                        last: true,
                    }]),
                };
                assert!(baseline.draw(cx, 600.) >= count);
            }
            let fixture = PreviewFixture {
                state: ListState::new(rows.len(), ListAlignment::Top, px(300.)),
                prepared,
                entity: entity.clone(),
                blocks,
                rows,
            };
            assert!(fixture.draw(cx, 600.) <= 96);
            for _ in 0..8 {
                let before = fixture.state.logical_scroll_top();
                scroll(cx, -120.);
                assert!(fixture.draw(cx, 600.) <= 96);
                let after = fixture.state.logical_scroll_top();
                assert!(
                    after.item_ix > before.item_ix
                        || (after.item_ix == before.item_ix
                            && after.offset_in_item > before.offset_in_item)
                );
            }
            for _ in 0..8 {
                let before = fixture.state.logical_scroll_top();
                scroll(cx, 120.);
                assert!(fixture.draw(cx, 600.) <= 96);
                let after = fixture.state.logical_scroll_top();
                assert!(
                    after.item_ix < before.item_ix
                        || (after.item_ix == before.item_ix
                            && after.offset_in_item < before.offset_in_item)
                );
            }
            let top = fixture.state.logical_scroll_top();
            assert_eq!(top.item_ix, 0);
            assert_eq!(top.offset_in_item, px(0.));
            // Jump deep into the block, resize, then scroll back upward.
            fixture.state.scroll_to(ListOffset {
                item_ix: fixture.rows.len() / 2,
                offset_in_item: px(0.),
            });
            assert!(fixture.draw(cx, 600.) <= 96);
            let before = fixture.state.logical_scroll_top();
            assert!(fixture.draw(cx, 320.) <= 96);
            assert_eq!(fixture.state.logical_scroll_top().item_ix, before.item_ix);
            scroll(cx, 120.);
            assert!(fixture.draw(cx, 320.) <= 96);
            assert!(fixture.state.logical_scroll_top().item_ix < before.item_ix);
            fixture.state.scroll_to(ListOffset {
                item_ix: fixture.rows.len() - 1,
                offset_in_item: px(0.),
            });
            assert!(fixture.draw(cx, 320.) <= 96);
            scroll(cx, -10_000.);
            assert!(fixture.draw(cx, 320.) <= 96);
            let bottom = fixture.state.logical_scroll_top();
            scroll(cx, -10_000.);
            assert!(fixture.draw(cx, 320.) <= 96);
            assert_eq!(fixture.state.logical_scroll_top().item_ix, bottom.item_ix);
            assert_eq!(
                fixture.state.logical_scroll_top().offset_in_item,
                bottom.offset_in_item
            );
        }
    }
}

#[gpui::test]
fn source_scroll_reuses_the_index_and_keeps_only_visible_hit_targets(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.select_all();
    state.insert_text(&"界 source line\n".repeat(10_000));
    let cx = cx.add_empty_window();
    let entity = cx.new(|cx| Emerald::with_state(state, cx));
    let draw = |cx: &mut VisualTestContext| {
        cx.draw(
            point(px(0.), px(0.)),
            size(px(600.), px(300.)),
            |window, cx| {
                let element = entity.update(cx, |view, cx| view.render_source(cx.entity(), window));
                cx.new(|_| DrawPreview(Some(element))).into_any_element()
            },
        );
        entity.read_with(cx, |view, _| {
            let geometry = view.interaction.source_geometry.borrow();
            assert!(!geometry.is_empty());
            assert!(geometry.len() < 40);
        });
    };
    draw(cx);
    draw(cx); // Adopt the measured wrapping width.
    let (list, index, layouts) = entity.read_with(cx, |view, _| {
        (
            view.source_list_state.clone(),
            view.interaction
                .source_lines_cache
                .as_ref()
                .unwrap()
                .ranges
                .clone(),
            view.interaction
                .source_lines_cache
                .as_ref()
                .unwrap()
                .layouts
                .clone(),
        )
    });
    for _ in 0..10 {
        scroll(cx, -84.);
        draw(cx);
    }
    assert!(list.logical_scroll_top().item_ix > 0);
    for _ in 0..10 {
        scroll(cx, 84.);
        draw(cx);
    }
    assert_eq!(list.logical_scroll_top().item_ix, 0);
    assert_eq!(list.logical_scroll_top().offset_in_item, px(0.));
    entity.read_with(cx, |view, _| {
        assert!(Arc::ptr_eq(
            &index,
            &view.interaction.source_lines_cache.as_ref().unwrap().ranges
        ));
        assert!(Rc::ptr_eq(
            &layouts,
            &view
                .interaction
                .source_lines_cache
                .as_ref()
                .unwrap()
                .layouts
        ));
        let geometry = view.interaction.source_geometry.borrow();
        assert!(geometry.iter().any(|row| row.start == 0));
        assert!(geometry.iter().all(|row| row.start < 1000));
    });
}

#[gpui::test]
fn source_layout_cache_survives_selection_but_not_edits_resize_or_file_switch(
    cx: &mut TestAppContext,
) {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.select_all();
    state.insert_text(&"界 source text ".repeat(40));
    let cx = cx.add_empty_window();
    let entity = cx.new(|cx| Emerald::with_state(state, cx));
    let draw = |cx: &mut VisualTestContext, width| {
        cx.draw(
            point(px(0.), px(0.)),
            size(px(width), px(300.)),
            |window, cx| {
                let element = entity.update(cx, |view, cx| view.render_source(cx.entity(), window));
                cx.new(|_| DrawPreview(Some(element))).into_any_element()
            },
        );
        entity.read_with(cx, |view, _| {
            view.interaction
                .source_lines_cache
                .as_ref()
                .unwrap()
                .layouts
                .clone()
        })
    };
    draw(cx, 600.);
    let original = draw(cx, 600.);
    entity.update(cx, |view, _| {
        view.state.set_cursor(0, false);
        view.state.set_cursor(3, true);
    });
    assert!(Rc::ptr_eq(&original, &draw(cx, 600.)));
    entity.update(cx, |view, _| view.state.insert_text("é"));
    let edited = draw(cx, 600.);
    assert!(!Rc::ptr_eq(&original, &edited));
    draw(cx, 320.);
    let resized = draw(cx, 320.);
    assert!(!Rc::ptr_eq(&edited, &resized));
    assert!(Rc::ptr_eq(&resized, &draw(cx, 320.)));
    entity.update(cx, |view, _| view.state.create_file("other.adoc").unwrap());
    assert!(!Rc::ptr_eq(&resized, &draw(cx, 320.)));
}

#[gpui::test]
fn source_reveal_follows_eof_across_edits_and_viewport_heights(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    for (height, trailing_newline) in [(112.0, false), (280.0, true)] {
        let temp = tempfile::tempdir().unwrap();
        let mut state = EditorState::open_or_create(temp.path()).unwrap();
        let mut text = "source line\n".repeat(30);
        if !trailing_newline {
            text.push_str("tail");
        }
        state.select_all();
        state.insert_text(&text);
        let entity = cx.new(|cx| Emerald::with_state(state, cx));
        let origin = (31.0, 47.0);
        let viewport = (420.0, height);

        request_source_reveal(&entity, cx);
        settle_source_reveal(cx, &entity, origin, viewport);
        assert_source_caret_visible(&entity, cx, origin.1, height);

        for insertion in ["\n", "continued", "\n"] {
            entity.update(cx, |view, _| view.state.insert_text(insertion));
            request_source_reveal(&entity, cx);
            settle_source_reveal(cx, &entity, origin, viewport);
            assert_source_caret_visible(&entity, cx, origin.1, height);
        }
    }
}

#[gpui::test]
fn source_reveal_uses_the_interior_caret_row_for_lines_and_soft_wraps(cx: &mut TestAppContext) {
    let cx = cx.add_empty_window();
    let origin = (24.0, 63.0);
    let viewport = (260.0, 140.0);

    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.select_all();
    state.insert_text(
        &(0..80)
            .map(|index| format!("source line {index:02}\n"))
            .collect::<String>(),
    );
    let entity = cx.new(|cx| Emerald::with_state(state, cx));
    request_source_reveal(&entity, cx);
    settle_source_reveal(cx, &entity, origin, viewport);

    entity.update(cx, |view, _| {
        let cursor = view.state.text().find("source line 08").unwrap() + 7;
        view.state.set_cursor(cursor, false);
        view.state.insert_text("X");
    });
    request_source_reveal(&entity, cx);
    settle_source_reveal(cx, &entity, origin, viewport);
    assert_source_caret_visible(&entity, cx, origin.1, viewport.1);
    entity.read_with(cx, |view, _| {
        let last = view
            .interaction
            .source_lines_cache
            .as_ref()
            .unwrap()
            .ranges
            .len()
            - 1;
        assert!(view.source_list_state.logical_scroll_top().item_ix < last);
    });

    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.select_all();
    state.insert_text(&"soft wrapped source text ".repeat(80));
    state.set_cursor(700, false);
    let wrapped = cx.new(|cx| Emerald::with_state(state, cx));
    request_source_reveal(&wrapped, cx);
    settle_source_reveal(cx, &wrapped, origin, viewport);
    assert_source_caret_visible(&wrapped, cx, origin.1, viewport.1);
}

#[gpui::test]
fn source_manual_scroll_stays_put_until_an_edit_requests_reveal(cx: &mut TestAppContext) {
    let temp = tempfile::tempdir().unwrap();
    let mut state = EditorState::open_or_create(temp.path()).unwrap();
    state.select_all();
    state.insert_text(&"source line\n".repeat(60));
    let cx = cx.add_empty_window();
    let entity = cx.new(|cx| Emerald::with_state(state, cx));
    let origin = (0.0, 0.0);
    let viewport = (420.0, 168.0);

    request_source_reveal(&entity, cx);
    settle_source_reveal(cx, &entity, origin, viewport);
    for _ in 0..20 {
        scroll(cx, 120.0);
        draw_source(cx, &entity, origin, viewport);
    }
    let scrolled = entity.read_with(cx, |view, _| {
        assert!(!view.interaction.source_cursor_reveal_pending);
        view.source_list_state.logical_scroll_top()
    });
    draw_source(cx, &entity, origin, viewport);
    draw_source(cx, &entity, origin, viewport);
    entity.read_with(cx, |view, _| {
        let current = view.source_list_state.logical_scroll_top();
        assert_eq!(current.item_ix, scrolled.item_ix);
        assert_eq!(current.offset_in_item, scrolled.offset_in_item);
    });

    entity.update(cx, |view, _| view.state.insert_text("continued"));
    request_source_reveal(&entity, cx);
    settle_source_reveal(cx, &entity, origin, viewport);
    assert_source_caret_visible(&entity, cx, origin.1, viewport.1);
    entity.read_with(cx, |view, _| {
        let current = view.source_list_state.logical_scroll_top();
        assert!(
            current.item_ix != scrolled.item_ix
                || current.offset_in_item != scrolled.offset_in_item
        );
    });
}
