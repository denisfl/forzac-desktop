//! Tables: column widths by content (a port of the Android app's `TableColumns`), rows of ragged cells, book-style rules.

use std::sync::Arc;

use super::{Align, BlockLayout, Cell, Content, Group, Spec, Typesetter};
use crate::fonts::Face;
use crate::model::Table;

/// Text sizes tried in turn when a table does not fit: the body size, then 0.9 and 0.8 of it.
const SCALES: [f32; 3] = [1.0, 0.9, 0.8];

/// Column positions and widths (relative to the text column) and the text scale a table is set at.
pub(super) struct Geometry {
    x: Vec<f32>,
    w: Vec<f32>,
    scale: f32,
}

/// Widths for columns whose longest word is `min` and whose text in one line is `max`, filling `available` (gaps
/// excluded). When the maxima fit, in proportion to them; when the minima fit, minimum plus the rest in proportion to
/// `max − min`; otherwise `None`.
pub(super) fn column_widths(min: &[f32], max: &[f32], available: f32) -> Option<Vec<f32>> {
    let maxes: Vec<f32> = max.iter().zip(min).map(|(&a, &b)| a.max(b).max(1.0)).collect();
    let sum_max: f32 = maxes.iter().sum();
    if sum_max <= available {
        return Some(maxes.iter().map(|m| m * available / sum_max).collect());
    }
    let sum_min: f32 = min.iter().sum();
    if sum_min > available {
        return None;
    }
    let slack = available - sum_min;
    let spread = sum_max - sum_min;
    Some(
        min.iter()
            .zip(&maxes)
            .map(|(&lo, &hi)| lo + if spread > 0.0 { slack * (hi - lo) / spread } else { slack / min.len() as f32 })
            .collect(),
    )
}

/// Widths when even the minima do not fit at the smallest size: in proportion to the minima; long words hyphenate.
pub(super) fn squeezed_widths(min: &[f32], available: f32) -> Vec<f32> {
    let sum = min.iter().sum::<f32>().max(1.0);
    min.iter().map(|m| m.max(1.0) * available / sum).collect()
}

impl Typesetter {
    fn geometry(&mut self, table: &Arc<Table>) -> Arc<Geometry> {
        let key = Arc::as_ptr(table) as usize;
        if let Some(g) = self.tables.get(&key) {
            return g.clone();
        }
        let columns = table.rows.iter().map(|r| r.iter().map(|c| c.col_span).sum::<usize>()).max().unwrap_or(1).max(1);
        let available = self.style.text_width();
        let mut geometry = None;
        for scale in SCALES {
            let size = self.style.text_size * scale;
            let gap = size;
            let inner = available - gap * (columns - 1) as f32;
            let mut min = vec![0f32; columns];
            let mut max = vec![0f32; columns];
            for row in &table.rows {
                let mut col = 0;
                for cell in row {
                    if cell.col_span == 1 && col < columns {
                        let face = Face::of(false, cell.header).id();
                        let text = &cell.text.text;
                        // +2: a word exactly as wide as its cell would otherwise wrap after rounding.
                        let word = text.split([' ', '\n', '\u{a0}']).map(|w| self.width_of(face, w, size)).fold(0.0, f32::max);
                        let line = text.split('\n').map(|l| self.width_of(face, l, size)).fold(0.0, f32::max);
                        min[col] = min[col].max(word + 2.0);
                        max[col] = max[col].max(line + 2.0);
                    }
                    col += cell.col_span;
                }
            }
            let widths =
                column_widths(&min, &max, inner).or_else(|| (scale == SCALES[SCALES.len() - 1]).then(|| squeezed_widths(&min, inner)));
            if let Some(w) = widths {
                let mut x = vec![0f32; columns];
                for i in 1..columns {
                    x[i] = x[i - 1] + w[i - 1] + gap;
                }
                geometry = Some(Geometry { x, w, scale });
                break;
            }
        }
        let geometry = Arc::new(geometry.expect("the smallest size always yields widths"));
        self.tables.insert(key, geometry.clone());
        geometry
    }

    /// Row `row` of `table`: cells as ragged paragraphs at the table's size, header cells bold, book-style rules.
    pub(super) fn table_row(&mut self, table: &Arc<Table>, row: usize) -> BlockLayout {
        let g = self.geometry(table);
        let columns = g.x.len();
        let pad = self.line_height * 0.25;
        let base = self.base_spec();
        let mut col = 0;
        let mut cells = Vec::new();
        for cell in &table.rows[row] {
            if col >= columns {
                break;
            }
            let span = cell.col_span.clamp(1, columns - col);
            let x = g.x[col];
            let width = g.x[col + span - 1] + g.w[col + span - 1] - x;
            col += span;
            let spec = Spec { width: width.max(1.0), align: Align::Left, justify: false, bold: cell.header, scale: g.scale, ..base };
            cells.push(Cell { text: self.text(&cell.text, &spec), x, y: pad });
        }
        let rows = &table.rows;
        let header = rows[row].iter().all(|c| c.header);
        let next_is_header = rows.get(row + 1).is_some_and(|r| r.iter().all(|c| c.header));
        let top = if row == 0 { 3 } else { 0 };
        let bottom = if row + 1 == rows.len() {
            3
        } else if header && !next_is_header {
            2
        } else {
            1
        };
        let height = cells.iter().map(|c| c.text.height()).fold(0.0, f32::max) + 2.0 * pad;
        let half = self.line_height / 2.0;
        BlockLayout {
            content: Content::Row { cells, rules: (top, bottom), height, width: self.style.text_width() },
            x: 0.0,
            group: Group::Table(Arc::as_ptr(table) as usize),
            space_before: half,
            space_after: half,
            keep_with_next: header,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths_follow_content() {
        // Maxima fit: in proportion to the maxima.
        assert_eq!(column_widths(&[10.0, 10.0], &[20.0, 60.0], 160.0), Some(vec![40.0, 120.0]));
        // Only minima fit: minimum plus the rest by max − min.
        let w = column_widths(&[20.0, 20.0], &[40.0, 120.0], 100.0).unwrap();
        assert!((w[0] - 30.0).abs() < 1e-3 && (w[1] - 70.0).abs() < 1e-3, "{w:?}");
        // Minima do not fit.
        assert_eq!(column_widths(&[80.0, 80.0], &[90.0, 90.0], 100.0), None);
        assert_eq!(squeezed_widths(&[80.0, 20.0], 50.0), vec![40.0, 10.0]);
    }
}
