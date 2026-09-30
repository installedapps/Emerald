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
        cx.draw(point(px(0.), px(0.)), size(px(width), px(300.)), |_, cx| {
            let element = list(self.state.clone(), move |index, _, _| {
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
                    &prepared,
                    std::path::Path::new("note.adoc"),
                    entity.clone(),
                )
            })
            .with_sizing_behavior(gpui::ListSizingBehavior::Auto)
            .w_full()
            .h_full()
            .into_any_element();
            cx.new(|_| DrawPreview(Some(element))).into_any_element()
        });
        let count = *work.borrow();
        count
    }
}

fn scroll(cx: &mut VisualTestContext, delta: f32) {
    cx.simulate_event(ScrollWheelEvent {
        position: point(px(100.), px(100.)),
        delta: ScrollDelta::Pixels(point(px(0.), px(delta))),
        ..Default::default()
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
