use super::*;

#[test]
fn everforest_palette_exposes_the_required_editor_regions() {
    assert_ne!(EVERFOREST_DARK.sidebar, EVERFOREST_DARK.window_background);
    assert_ne!(EVERFOREST_DARK.panel, EVERFOREST_DARK.sidebar);
    assert_ne!(EVERFOREST_DARK.accent, EVERFOREST_DARK.text);
    assert_ne!(EVERFOREST_DARK.active_file, EVERFOREST_DARK.sidebar);
}
