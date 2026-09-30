use super::*;

#[test]
fn idle_preview_waits_for_selection_to_be_cleared() {
    assert!(!source_can_return_to_preview(
        false,
        Some(&emerald::Selection { start: 1, end: 4 })
    ));
    assert!(!source_can_return_to_preview(true, None));
    assert!(source_can_return_to_preview(false, None));
}
