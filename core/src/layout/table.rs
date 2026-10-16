//! Tables: column widths by content (a port of the Android app's `TableColumns`), rows of ragged cells, book-style rules.

use std::sync::Arc;

use super::page::draw_block;
use super::{Align, BlockLayout, Cell, Content, Group, PageContent, Spec, Typesetter};
use crate::fonts::Face;
use crate::model::{Block, Book, Locator, Table};

/// Text sizes tried in turn when a table does not fit: the body size, then 0.9 and 0.8 of it.
const SCALES: [f32; 3] = [1.0, 0.9, 0.8];

/// The widest column in the view, em: longer text wraps in its cell (Android's `TableViewer`).
const MAX_COLUMN: f32 = 20.0;

/// Column positions and widths (relative to the text column), the table's width and the text scale it is set at.
pub(super) struct Geometry {
    pub(super) x: Vec<f32>,
    pub(super) w: Vec<f32>,
    pub(super) width: f32,
    scale: f32,
}

impl Geometry {
    /// Columns of widths `w` with `gap` between them, `width` wide in all.
    fn new(w: Vec<f32>, gap: f32, width: f32, scale: f32) -> Self {
        let mut x = vec![0f32; w.len()];
        for i in 1..w.len() {
            x[i] = x[i - 1] + w[i - 1] + gap;
        }
        Geometry { x, w, width, scale }
    }
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

/// Widths for the view: each column as wide as its longest line up to `cap`, never narrower than its longest word.
pub(super) fn natural_widths(min: &[f32], max: &[f32], cap: f32) -> Vec<f32> {
    min.iter().zip(max).map(|(&lo, &hi)| lo.max(hi.min(cap)).max(1.0)).collect()
}

/// A table typeset for the view: its drawing (rules, text, the areas of its note markers and links) from the origin,
/// and its size.
#[derive(Debug, Clone, Default)]
pub struct TableView {
    pub content: PageContent,
    pub width: f32,
    pub height: f32,
}

/// The table whose first row is at `at`, typeset for the view at the typesetter's text size: columns by content
/// (`natural_widths`), not fitted to the column; rows one under another. `None` when `at` is not a table's first row.
pub fn layout_table(book: &Book, at: Locator, typesetter: &mut Typesetter) -> Option<TableView> {
    let Block::TableRow { table, row: 0 } = book.chapters.get(at.chapter)?.blocks.get(at.block)? else { return None };
    // The rows take their geometry from the cache: the view's for the time of this call.
    let key = Arc::as_ptr(table) as usize;
    let geometry = Arc::new(typesetter.natural_geometry(table));
    let page_geometry = typesetter.tables.insert(key, geometry.clone());
    let mut view = TableView { width: geometry.width, ..TableView::default() };
    for row in 0..table.rows.len() {
        let layout = typesetter.table_row(table, row);
        draw_block(&mut view.content, &layout, 0..1, 0.0, view.height);
        view.height += layout.height();
    }
    match page_geometry {
        Some(g) => typesetter.tables.insert(key, g),
        None => typesetter.tables.remove(&key),
    };
    Some(view)
}

impl Typesetter {
    fn columns(table: &Table) -> usize {
        table.rows.iter().map(|r| r.iter().map(|c| c.col_span).sum::<usize>()).max().unwrap_or(1).max(1)
    }

    /// The longest word and the longest line (+2: a word exactly as wide as its cell would otherwise wrap after
    /// rounding) of each column's cells that span one column, at `size`.
    fn measure(&mut self, table: &Table, columns: usize, size: f32) -> (Vec<f32>, Vec<f32>) {
        let mut min = vec![0f32; columns];
        let mut max = vec![0f32; columns];
        for row in &table.rows {
            let mut col = 0;
            for cell in row {
                if cell.col_span == 1 && col < columns {
                    let face = self.font_of(Face::of(false, cell.header));
                    let text = &cell.text.text;
                    let word = text.split([' ', '\n', '\u{a0}']).map(|w| self.width_of(face, w, size)).fold(0.0, f32::max);
                    let line = text.split('\n').map(|l| self.width_of(face, l, size)).fold(0.0, f32::max);
                    min[col] = min[col].max(word + 2.0);
                    max[col] = max[col].max(line + 2.0);
                }
                col += cell.col_span;
            }
        }
        (min, max)
    }

    fn geometry(&mut self, table: &Arc<Table>) -> Arc<Geometry> {
        let key = Arc::as_ptr(table) as usize;
        if let Some(g) = self.tables.get(&key) {
            return g.clone();
        }
        let columns = Self::columns(table);
        let available = self.style.text_width();
        let mut geometry = None;
        for scale in SCALES {
            let size = self.style.text_size * scale;
            let gap = size;
            let inner = available - gap * (columns - 1) as f32;
            let (min, max) = self.measure(table, columns, size);
            let widths =
                column_widths(&min, &max, inner).or_else(|| (scale == SCALES[SCALES.len() - 1]).then(|| squeezed_widths(&min, inner)));
            if let Some(w) = widths {
                geometry = Some(Geometry::new(w, gap, available, scale));
                break;
            }
        }
        let geometry = Arc::new(geometry.expect("the smallest size always yields widths"));
        self.tables.insert(key, geometry.clone());
        geometry
    }

    /// The view's geometry: `natural_widths` at the text size, the page's gap of 1 em between columns.
    pub(super) fn natural_geometry(&mut self, table: &Table) -> Geometry {
        let columns = Self::columns(table);
        let size = self.style.text_size;
        let (min, max) = self.measure(table, columns, size);
        let w = natural_widths(&min, &max, size * MAX_COLUMN);
        let width = w.iter().sum::<f32>() + size * (columns - 1) as f32;
        Geometry::new(w, size, width, 1.0)
    }

    /// Row `row` of `table`: cells as ragged paragraphs at the table's size, header cells bold, book-style rules.
    pub(super) fn table_row(&mut self, table: &Arc<Table>, row: usize) -> BlockLayout {
        let g = self.geometry(table);
        let columns = g.x.len();
        let pad = self.line_height * 0.25;
        let base = self.base_spec();
        let mut col = 0;
        let mut cells = Vec::new();
        let starts = table.cell_starts(row);
        for (cell, offset) in table.rows[row].iter().zip(starts) {
            if col >= columns {
                break;
            }
            let span = cell.col_span.clamp(1, columns - col);
            let x = g.x[col];
            let width = g.x[col + span - 1] + g.w[col + span - 1] - x;
            col += span;
            let spec = Spec { width: width.max(1.0), align: Align::Left, justify: false, bold: cell.header, scale: g.scale, ..base };
            cells.push(Cell { text: self.text(&cell.text, &spec), x, y: pad, offset });
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
            content: Content::Row { row, cells, rules: (top, bottom), height, width: g.width },
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
        // The view: the longest line up to the cap, never below the longest word.
        assert_eq!(natural_widths(&[10.0, 10.0, 500.0], &[50.0, 900.0, 900.0], 400.0), vec![50.0, 400.0, 500.0]);
    }
}
