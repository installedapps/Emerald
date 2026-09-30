use super::*;

#[test]
fn select_all_recognizes_control_and_platform_shortcuts_only() {
    for key in ["ctrl-a", "cmd-a"] {
        assert!(is_select_all(&Keystroke::parse(key).unwrap()));
    }
    for key in ["a", "alt-a", "ctrl-alt-a", "ctrl-b"] {
        assert!(!is_select_all(&Keystroke::parse(key).unwrap()));
    }
}

#[test]
fn renaming_uses_typed_capitals_spaces_and_unicode() {
    let mut name = String::new();
    for (key, text) in [("shift-p", "P"), ("space", " "), ("e", "é")] {
        let mut keystroke = Keystroke::parse(key).unwrap();
        keystroke.key_char = Some(text.into());
        edit_rename_buffer(&mut name, &keystroke);
    }
    assert_eq!(name, "P é");
    edit_rename_buffer(&mut name, &Keystroke::parse("backspace").unwrap());
    assert_eq!(name, "P ");
    let mut shortcut = Keystroke::parse("ctrl-a").unwrap();
    shortcut.key_char = Some("a".into());
    edit_rename_buffer(&mut name, &shortcut);
    assert_eq!(name, "P ");
}
