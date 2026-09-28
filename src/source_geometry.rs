use std::ops::Range;

/// Byte boundaries measured from the exact shaped line that is painted.
/// Window-space origins already include panel layout and scrolling.
#[derive(Clone, Debug)]
pub(super) struct SourceRowGeometry {
    pub start: usize,
    pub origin: (f32, f32),
    pub height: f32,
    pub stops: std::sync::Arc<[(usize, f32)]>,
}

impl SourceRowGeometry {
    pub fn x_for_offset(&self, offset: usize) -> f32 {
        self.stops
            .iter()
            .find(|(index, _)| *index >= offset)
            .or_else(|| self.stops.last())
            .map_or(0.0, |(_, x)| *x)
    }

    pub fn selection_span(&self, range: Range<usize>) -> Option<(f32, f32)> {
        (range.start < range.end)
            .then(|| (self.x_for_offset(range.start), self.x_for_offset(range.end)))
    }
}

pub(super) fn source_offset_at(rows: &[SourceRowGeometry], pointer: (f32, f32)) -> Option<usize> {
    let row = rows.iter().min_by(|a, b| {
        let distance = |row: &SourceRowGeometry| {
            (row.origin.1 - pointer.1).max(0.0) + (pointer.1 - (row.origin.1 + row.height)).max(0.0)
        };
        distance(a)
            .total_cmp(&distance(b))
            // At the shared boundary, the next row owns the pointer.
            .then_with(|| b.origin.1.total_cmp(&a.origin.1))
    })?;
    let x = pointer.0 - row.origin.0;
    let (index, _) = row
        .stops
        .iter()
        .min_by(|a, b| (a.1 - x).abs().total_cmp(&(b.1 - x).abs()))?;
    Some(row.start + index)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(start: usize, x: f32, y: f32) -> SourceRowGeometry {
        // Measured boundaries for "a界b": the middle glyph is twice as wide.
        SourceRowGeometry {
            start,
            origin: (x, y),
            height: 28.0,
            stops: vec![(0, 0.0), (1, 10.0), (4, 30.0), (5, 40.0)].into(),
        }
    }

    #[test]
    fn soft_wrapped_rows_hit_document_offsets_at_the_shared_boundary() {
        let rows = vec![row(0, 30.0, 50.0), row(5, 30.0, 78.0)];
        assert_eq!(source_offset_at(&rows, (30.0, 78.0)), Some(5));
        assert_eq!(source_offset_at(&rows, (60.0, 80.0)), Some(9));
    }

    #[test]
    fn selection_hit_testing_uses_painted_origins_in_every_layout_and_scroll_position() {
        for (x, y) in [(325.0, 98.0), (37.0, 280.0), (275.0, 70.0)] {
            let rows = vec![row(12, x, y), row(18, x, y + 28.0)];
            assert_eq!(source_offset_at(&rows, (x + 10.0, y + 5.0)), Some(13));
            assert_eq!(source_offset_at(&rows, (x + 10.0, y + 33.0)), Some(19));
            assert_eq!(source_offset_at(&rows, (x - 90.0, y + 5.0)), Some(12));
        }
    }

    #[test]
    fn unicode_and_end_of_line_hits_choose_the_nearest_measured_boundary() {
        let rows = vec![row(0, 325.0, 98.0)];
        assert_eq!(source_offset_at(&rows, (344.0, 100.0)), Some(1));
        assert_eq!(source_offset_at(&rows, (346.0, 100.0)), Some(4));
        assert_eq!(source_offset_at(&rows, (356.0, 100.0)), Some(4));
        assert_eq!(source_offset_at(&rows, (364.0, 100.0)), Some(5));
        assert_eq!(source_offset_at(&rows, (900.0, 100.0)), Some(5));
    }

    #[test]
    fn drag_outside_visible_rows_clamps_to_the_nearest_row() {
        let rows = vec![row(12, 325.0, 98.0), row(18, 325.0, 126.0)];
        assert_eq!(source_offset_at(&rows, (335.0, 0.0)), Some(13));
        assert_eq!(source_offset_at(&rows, (335.0, 900.0)), Some(19));
        assert_eq!(source_offset_at(&[], (0.0, 0.0)), None);
    }

    #[test]
    fn caret_and_selection_geometry_do_not_move_text_boundaries() {
        let row = row(0, 325.0, 98.0);
        assert_eq!(row.x_for_offset(1), 10.0);
        assert_eq!(row.x_for_offset(4), 30.0);
        assert_eq!(row.selection_span(1..4), Some((10.0, 30.0)));
        assert_eq!(row.selection_span(2..2), None);
        assert_eq!(row.x_for_offset(5), 40.0);
    }

    #[test]
    fn dragged_unicode_range_matches_highlighting_and_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let mut editor = emerald::EditorState::open_or_create(temp.path()).unwrap();
        editor.select_all();
        editor.insert_text("a界b\na界b");
        let rows = vec![row(0, 325.0, 98.0), row(6, 325.0, 126.0)];
        editor.begin_mouse_selection(source_offset_at(&rows, (335.0, 100.0)).unwrap());
        editor.update_mouse_selection(source_offset_at(&rows, (355.0, 130.0)).unwrap());
        editor.end_mouse_selection();
        assert_eq!(editor.selected_text(), Some("界b\na界"));
        let highlighted =
            emerald::ui::source_view_lines(editor.text(), editor.cursor(), editor.selection());
        let selected: Vec<_> = highlighted
            .iter()
            .map(|line| {
                line.segments
                    .iter()
                    .filter(|segment| segment.selected)
                    .map(|segment| segment.text.as_str())
                    .collect::<String>()
            })
            .collect();
        assert_eq!(selected, vec!["界b", "a界"]);
        assert!(highlighted[0].newline_selected);
        assert!(!highlighted[1].newline_selected);
        editor.insert_text("X");
        assert_eq!(editor.text(), "aXb");
        assert_eq!(editor.cursor(), 2);
        assert_eq!(editor.selection(), None);
    }
}
