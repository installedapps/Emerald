use super::*;

#[test]
fn soft_wraps_preserve_unicode_selection_and_own_the_caret_once() {
    let rows =
        wrapped_source_view_lines("a界b c\n", 4, Some(Selection { start: 1, end: 8 }), |_| {
            vec![4]
        });
    assert_eq!(
        rows.iter()
            .map(|row| (row.start, row.end))
            .collect::<Vec<_>>(),
        vec![(0, 4), (4, 7), (8, 8)]
    );
    assert!(!rows[0].cursor_at_end);
    assert!(rows[1].segments[0].cursor_before);
    assert!(!rows[0].newline_selected);
    assert!(rows[1].newline_selected);
    let selected: String = rows
        .iter()
        .flat_map(|row| &row.segments)
        .filter(|segment| segment.selected)
        .map(|segment| segment.text.as_str())
        .collect();
    assert_eq!(selected, "界b c");
    assert!(wrapped_source_view_lines("", 0, None, |_| vec![])[0].cursor_at_end);
}
