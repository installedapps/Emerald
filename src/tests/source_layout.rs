use super::*;

#[gpui::test]
fn stops_match_gpui_for_unicode_ligatures_and_empty_text(cx: &mut gpui::TestAppContext) {
    let cx = cx.add_empty_window();
    cx.update(|window, _| {
        for text in [
            "",
            "abc",
            "界é🙂",
            "office ffi",
            "e\u{301}\u{200d}🙂",
            "مرحبا abc",
        ] {
            let shaped = window.text_system().shape_line(
                text.to_owned().into(),
                px(18.0),
                &[text_run(text.len(), emerald::theme::EVERFOREST_DARK.text)],
                None,
            );
            for (index, x) in character_stops(text, &shaped) {
                assert_eq!(x, f32::from(shaped.x_for_index(index)));
            }
        }
    });
}

#[gpui::test]
fn cache_reuses_measurements_and_bounds_retained_text(cx: &mut gpui::TestAppContext) {
    let cx = cx.add_empty_window();
    cx.update(|window, _| {
        let mut cache = SourceLayoutCache::default();
        let first = cache.get_or_prepare(
            0,
            "界 source\n",
            0,
            200.,
            emerald::theme::EVERFOREST_DARK.text,
            window,
        );
        assert!(Rc::ptr_eq(
            &first,
            &cache.get_or_prepare(
                0,
                "界 source\n",
                0,
                200.,
                emerald::theme::EVERFOREST_DARK.text,
                window,
            )
        ));
        let text = "a".repeat(1024);
        for index in 1..200 {
            cache.get_or_prepare(
                index,
                &text,
                index * text.len(),
                200.,
                emerald::theme::EVERFOREST_DARK.text,
                window,
            );
            assert!(cache.lines.len() <= MAX_LINES);
            assert!(cache.text_bytes <= MAX_TEXT_BYTES);
        }
        assert!(!Rc::ptr_eq(
            &first,
            &cache.get_or_prepare(
                0,
                "界 source\n",
                0,
                200.,
                emerald::theme::EVERFOREST_DARK.text,
                window,
            )
        ));
    });
}
