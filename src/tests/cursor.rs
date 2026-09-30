use super::*;

#[test]
fn cursor_toggles_visibility_on_tick() {
    let mut cursor = BlinkCursor::default();

    assert!(cursor.visible());
    cursor.tick();
    assert!(!cursor.visible());
    cursor.tick();
    assert!(cursor.visible());
}

#[test]
fn cursor_uses_a_human_readable_blink_interval() {
    let cursor = BlinkCursor::default();

    assert_eq!(cursor.interval(), Duration::from_millis(530));
}
