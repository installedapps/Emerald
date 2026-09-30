use super::admonition_colors;

#[test]
fn admonition_kinds_have_distinct_theme_accents() {
    let kinds = ["NOTE", "TIP", "IMPORTANT", "CAUTION", "WARNING"];
    let colors = kinds
        .iter()
        .map(|kind| admonition_colors(kind, 0).0)
        .collect::<Vec<_>>();
    assert_eq!(
        colors.windows(2).filter(|pair| pair[0] != pair[1]).count(),
        4
    );
    assert_eq!(admonition_colors("custom", 0x123456).0, 0x123456);
}
