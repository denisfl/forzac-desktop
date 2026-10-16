//! Knuth–Plass line breaking: the break points that minimise the demerits of the whole paragraph, which is what makes
//! justified text even. Measurement-agnostic: widths come from the caller.

pub const INFINITY: f32 = 10_000.0;

#[derive(Debug, Clone, Copy)]
pub enum Item {
    /// Unbreakable content: a syllable, a word, the first-line indent.
    Box { width: f32 },
    /// A space that stretches and shrinks.
    Glue { width: f32, stretch: f32, shrink: f32 },
    /// A possible break; `width` is added only when the line breaks here (a hyphen). `flagged`: a hyphenation.
    Penalty { width: f32, penalty: f32, flagged: bool },
}

impl Item {
    fn width(&self) -> f32 {
        match *self {
            Item::Box { width } | Item::Glue { width, .. } | Item::Penalty { width, .. } => width,
        }
    }
}

/// A break: the item index and the adjustment ratio of the line that ends there.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Break {
    pub index: usize,
    pub ratio: f32,
}

struct Node {
    index: usize,
    line: usize,
    fitness: usize,
    /// Totals after the break, i.e. at the start of the next line.
    width: f32,
    stretch: f32,
    shrink: f32,
    demerits: f32,
    ratio: f32,
    prev: Option<usize>,
}

/// Breaks `items` into lines of `line_width(line)`. Tries tolerance 1, then 3, then 10; if nothing fits (a word wider
/// than the line), falls back to first-fit.
pub fn break_lines(items: &[Item], line_width: impl Fn(usize) -> f32) -> Vec<Break> {
    for tolerance in [1.0, 3.0, 10.0] {
        if let Some(b) = optimal(items, &line_width, tolerance) {
            return b;
        }
    }
    greedy(items, &line_width)
}

fn breakable(items: &[Item], i: usize) -> bool {
    match items[i] {
        Item::Box { .. } => false,
        Item::Glue { .. } => i > 0 && matches!(items[i - 1], Item::Box { .. }),
        Item::Penalty { penalty, .. } => penalty < INFINITY,
    }
}

fn optimal(items: &[Item], line_width: &impl Fn(usize) -> f32, tolerance: f32) -> Option<Vec<Break>> {
    let mut nodes =
        vec![Node { index: 0, line: 0, fitness: 1, width: 0.0, stretch: 0.0, shrink: 0.0, demerits: 0.0, ratio: 0.0, prev: None }];
    let mut active: Vec<usize> = vec![0];
    let (mut sum_w, mut sum_y, mut sum_z) = (0.0f32, 0.0f32, 0.0f32);
    for i in 0..items.len() {
        let item = items[i];
        if breakable(items, i) {
            let (pen_w, pen, flagged) = match item {
                Item::Penalty { width, penalty, flagged } => (width, penalty, flagged),
                _ => (0.0, 0.0, false),
            };
            let mut best: [Option<(f32, usize, f32)>; 4] = [None; 4]; // per fitness class: demerits, from node, ratio
            active.retain(|&a| {
                let n = &nodes[a];
                let target = line_width(n.line);
                let w = sum_w - n.width + pen_w;
                let ratio = if w < target {
                    let y = sum_y - n.stretch;
                    if y > 0.0 { (target - w) / y } else { INFINITY }
                } else if w > target {
                    let z = sum_z - n.shrink;
                    if z > 0.0 { (target - w) / z } else { -INFINITY }
                } else {
                    0.0
                };
                let keep = ratio >= -1.0 && pen > -INFINITY;
                if (-1.0..=tolerance).contains(&ratio) {
                    let badness = 100.0 * ratio.abs().powi(3);
                    let mut d = if pen >= 0.0 {
                        (1.0 + badness + pen).powi(2)
                    } else if pen > -INFINITY {
                        (1.0 + badness).powi(2) - pen * pen
                    } else {
                        (1.0 + badness).powi(2)
                    };
                    let prev_flagged = matches!(items.get(n.index), Some(Item::Penalty { flagged: true, .. })) && n.prev.is_some();
                    if flagged && prev_flagged {
                        d += 3000.0; // two hyphens in a row
                    }
                    let fitness: usize = if ratio < -0.5 {
                        0
                    } else if ratio <= 0.5 {
                        1
                    } else if ratio <= 1.0 {
                        2
                    } else {
                        3
                    };
                    if n.prev.is_some() && fitness.abs_diff(n.fitness) > 1 {
                        d += 3000.0; // a tight line next to a loose one
                    }
                    let total = n.demerits + d;
                    if best[fitness].is_none_or(|(b, _, _)| total < b) {
                        best[fitness] = Some((total, a, ratio));
                    }
                }
                keep
            });
            // Totals after a break here: skip the glue and penalties up to the next box.
            let (mut w, mut y, mut z) = (sum_w, sum_y, sum_z);
            for (j, next) in items.iter().enumerate().skip(i) {
                match *next {
                    Item::Box { .. } => break,
                    Item::Penalty { penalty, .. } if penalty <= -INFINITY && j > i => break,
                    Item::Glue { width, stretch, shrink } => {
                        w += width;
                        y += stretch;
                        z += shrink;
                    }
                    Item::Penalty { .. } => {}
                }
            }
            for (fitness, b) in best.iter().enumerate() {
                if let Some((demerits, from, ratio)) = *b {
                    nodes.push(Node {
                        index: i,
                        line: nodes[from].line + 1,
                        fitness,
                        width: w,
                        stretch: y,
                        shrink: z,
                        demerits,
                        ratio,
                        prev: Some(from),
                    });
                    active.push(nodes.len() - 1);
                }
            }
            if active.is_empty() {
                return None;
            }
        }
        match item {
            Item::Box { width } => sum_w += width,
            Item::Glue { width, stretch, shrink } => {
                sum_w += width;
                sum_y += stretch;
                sum_z += shrink;
            }
            Item::Penalty { .. } => {}
        }
    }
    let last = items.len().checked_sub(1)?;
    let best = active
        .iter()
        .copied()
        .filter(|&a| nodes[a].index == last && nodes[a].prev.is_some())
        .min_by(|&a, &b| nodes[a].demerits.total_cmp(&nodes[b].demerits))?;
    let mut out = Vec::new();
    let mut at = Some(best);
    while let Some(n) = at {
        if nodes[n].prev.is_none() {
            break;
        }
        out.push(Break { index: nodes[n].index, ratio: nodes[n].ratio });
        at = nodes[n].prev;
    }
    out.reverse();
    Some(out)
}

/// First fit; a box wider than the line sits on a line of its own.
fn greedy(items: &[Item], line_width: &impl Fn(usize) -> f32) -> Vec<Break> {
    let mut out = Vec::new();
    let mut w = 0.0;
    let mut last_break: Option<usize> = None;
    for (i, item) in items.iter().enumerate() {
        if let Item::Penalty { penalty, .. } = item
            && *penalty <= -INFINITY
        {
            out.push(Break { index: i, ratio: 0.0 });
            break;
        }
        let can = breakable(items, i);
        let pen_w = if let Item::Penalty { width, .. } = item { *width } else { 0.0 };
        if can && w + pen_w > line_width(out.len()) {
            let at = last_break.unwrap_or(i);
            out.push(Break { index: at, ratio: 0.0 });
            last_break = None;
            w = 0.0;
            if at == i {
                continue;
            }
            for it in &items[at + 1..i] {
                if matches!(it, Item::Box { .. }) || (w > 0.0 && matches!(it, Item::Glue { .. })) {
                    w += it.width();
                }
            }
        }
        if can {
            last_break = Some(i);
        }
        if !matches!(item, Item::Penalty { .. }) {
            w += item.width();
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn paragraph(words: usize, word: f32) -> Vec<Item> {
        let mut items = Vec::new();
        for i in 0..words {
            if i > 0 {
                items.push(Item::Glue { width: 4.0, stretch: 2.0, shrink: 1.33 });
            }
            items.push(Item::Box { width: word });
        }
        items.push(Item::Penalty { width: 0.0, penalty: INFINITY, flagged: false });
        items.push(Item::Glue { width: 0.0, stretch: 100_000.0, shrink: 0.0 });
        items.push(Item::Penalty { width: 0.0, penalty: -INFINITY, flagged: false });
        items
    }

    #[test]
    fn lines_fit_within_tolerance() {
        let items = paragraph(40, 10.0);
        let breaks = break_lines(&items, |_| 100.0);
        assert_eq!(breaks.last().unwrap().index, items.len() - 1);
        for b in &breaks[..breaks.len() - 1] {
            assert!((-1.0..=1.0).contains(&b.ratio), "ratio {}", b.ratio);
        }
    }

    #[test]
    fn word_wider_than_line_still_breaks() {
        let mut items = paragraph(2, 10.0);
        items[0] = Item::Box { width: 300.0 };
        assert_eq!(break_lines(&items, |_| 100.0).len(), 2);
    }
}
