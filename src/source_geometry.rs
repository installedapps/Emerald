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
#[path = "tests/source_geometry.rs"]
mod tests;
