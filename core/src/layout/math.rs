//! A formula typeset by simplified TeX rules, a port of Android's `MathLayout`: scripts and a fraction's parts at 0.71 of
//! the parent's size (at least half the formula's); a fraction's rule on the axis (0.25 em); spaces around binary operators
//! and relations, none at a unary one; the limits of ∑ ∏ ∫ at the side in a line, above and below in a display formula;
//! stretched fences and a root sign as polylines.

use crate::math::{ACCENT_MARKS, BARS, ColumnAlign, MathNode, OperatorClass, TokenKind, Variant, operator_class};

/// A token's width and its height above and below the baseline, by its glyphs.
pub(super) trait MathMetrics {
    fn measure(&mut self, text: &str, size: f32, italic: bool, bold: bool) -> (f32, f32, f32);
}

/// A part of a typeset formula: from the box's left edge and baseline, y down.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum MathItem {
    /// Text with its baseline at `y`.
    Text { text: String, x: f32, y: f32, size: f32, italic: bool, bold: bool },
    /// A filled rectangle: a fraction's rule, a bar, a line of a matrix.
    Rule { x: f32, y: f32, w: f32, h: f32 },
    /// A polyline: a root sign, a stretched fence.
    Path { points: Vec<[f32; 2]>, width: f32 },
}

impl MathItem {
    fn shifted(&self, dx: f32, dy: f32) -> MathItem {
        match self {
            MathItem::Text { text, x, y, size, italic, bold } => {
                MathItem::Text { text: text.clone(), x: x + dx, y: y + dy, size: *size, italic: *italic, bold: *bold }
            }
            MathItem::Rule { x, y, w, h } => MathItem::Rule { x: x + dx, y: y + dy, w: *w, h: *h },
            MathItem::Path { points, width } => {
                MathItem::Path { points: points.iter().map(|p| [p[0] + dx, p[1] + dy]).collect(), width: *width }
            }
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(super) struct MathBox {
    pub width: f32,
    pub ascent: f32,
    pub descent: f32,
    pub items: Vec<MathItem>,
}

impl MathBox {
    fn shifted(&self, dx: f32, dy: f32) -> Vec<MathItem> {
        self.items.iter().map(|i| i.shifted(dx, dy)).collect()
    }
}

const OPENING: &[&str] = &["(", "[", "{", "⟨", "|", "‖"];

fn is_large(node: &MathNode) -> bool {
    matches!(node, MathNode::Token { text, kind: TokenKind::Operator, .. } if operator_class(text) == OperatorClass::Large)
}

fn fence_token(node: &MathNode) -> Option<&str> {
    match node {
        MathNode::Token { text, kind: TokenKind::Operator, .. } if operator_class(text) == OperatorClass::Fence => Some(text),
        _ => None,
    }
}

fn italic(node: &MathNode) -> bool {
    match node {
        MathNode::Token { text, kind, variant, .. } => match variant {
            Variant::Italic | Variant::BoldItalic => true,
            Variant::Normal | Variant::Bold => false,
            Variant::Default => *kind == TokenKind::Identifier && text.chars().count() == 1 && text.chars().all(char::is_alphabetic),
        },
        _ => false,
    }
}

/// `size` is the formula's text size, `display` whether it is a display formula.
pub(super) struct MathLayout<'a, M: MathMetrics> {
    metrics: &'a mut M,
    size: f32,
    display: bool,
}

impl<'a, M: MathMetrics> MathLayout<'a, M> {
    pub fn new(metrics: &'a mut M, size: f32, display: bool) -> Self {
        MathLayout { metrics, size, display }
    }

    pub fn layout(&mut self, node: &MathNode) -> MathBox {
        self.node(node, self.size, 0)
    }

    fn script(&self, s: f32) -> f32 {
        (s * 0.71).max(self.size * 0.5)
    }

    fn node(&mut self, node: &MathNode, s: f32, level: u32) -> MathBox {
        match node {
            MathNode::Token { .. } => self.token(node, s),
            MathNode::Row(children) => self.row(children, s, level),
            MathNode::Frac { num, den, line } => self.frac(num, den, *line, s, level),
            MathNode::Scripts { base, sub, sup } => {
                let b = self.node(base, s, level);
                self.scripts(b, sub.as_deref(), sup.as_deref(), s, level, italic(base))
            }
            MathNode::UnderOver { base, under, over, accent_over } => {
                self.under_over(base, under.as_deref(), over.as_deref(), *accent_over, s, level)
            }
            MathNode::Root { radicand, index } => self.root(radicand, index.as_deref(), s, level),
            MathNode::Fenced { open, close, content } => {
                let c = self.node(content, s, level);
                self.fenced(open, c, close, s)
            }
            MathNode::Matrix { rows, align, lines } => self.matrix(rows, align, lines, s, level),
            MathNode::Space(em) => MathBox { width: em * s, ..MathBox::default() },
        }
    }

    fn token(&mut self, node: &MathNode, s: f32) -> MathBox {
        let MathNode::Token { text, kind, variant, scale } = node else { return MathBox::default() };
        let size = s * scale;
        let it = italic(node);
        let bold = matches!(variant, Variant::Bold | Variant::BoldItalic);
        // A display formula's large operator is larger.
        let glyph =
            if self.display && *kind == TokenKind::Operator && operator_class(text) == OperatorClass::Large { size * 1.4 } else { size };
        let (w, ascent, descent) = self.metrics.measure(text, glyph, it, bold);
        MathBox {
            width: w + if it { glyph * 0.04 } else { 0.0 },
            ascent,
            descent,
            items: vec![MathItem::Text { text: text.clone(), x: 0.0, y: 0.0, size: glyph, italic: it, bold }],
        }
    }

    /// A row: parts along the baseline, spaces at operators; fences among them stretch to the row's contents.
    fn row(&mut self, children: &[MathNode], s: f32, level: u32) -> MathBox {
        if children.is_empty() {
            return MathBox::default();
        }
        let mut boxes: Vec<Option<MathBox>> = children.iter().map(|c| fence_token(c).is_none().then(|| self.node(c, s, level))).collect();
        let ascent = boxes.iter().flatten().map(|b| b.ascent).fold(0.0, f32::max);
        let descent = boxes.iter().flatten().map(|b| b.descent).fold(0.0, f32::max);
        for (slot, child) in boxes.iter_mut().zip(children) {
            if let Some(text) = fence_token(child) {
                *slot = Some(self.delimiter(text, ascent, descent, s));
            }
        }
        // Spaces are half as wide in scripts: the formula is small there already.
        let k = if level > 0 { 0.5 } else { 1.0 };
        let mut out = MathBox::default();
        let mut x = 0.0;
        for (i, b) in boxes.iter().flatten().enumerate() {
            let (before, after) = spacing(children, i, s * k);
            x += before;
            out.items.extend(b.shifted(x, 0.0));
            x += b.width + after;
            out.ascent = out.ascent.max(b.ascent);
            out.descent = out.descent.max(b.descent);
        }
        out.width = x;
        out
    }

    fn frac(&mut self, num: &MathNode, den: &MathNode, line: bool, s: f32, level: u32) -> MathBox {
        let inner = if self.display && level == 0 { s } else { self.script(s) };
        let (num, den) = (self.node(num, inner, level + 1), self.node(den, inner, level + 1));
        let t = (s * 0.06).max(1.0);
        let (gap, pad) = (s * 0.12, s * 0.1);
        let width = num.width.max(den.width) + 2.0 * pad;
        let a = s * 0.25;
        let num_y = -a - t / 2.0 - gap - num.descent;
        let den_y = -a + t / 2.0 + gap + den.ascent;
        let mut items = num.shifted((width - num.width) / 2.0, num_y);
        items.extend(den.shifted((width - den.width) / 2.0, den_y));
        if line {
            items.push(MathItem::Rule { x: pad / 2.0, y: -a - t / 2.0, w: width - pad, h: t });
        }
        MathBox { width, ascent: -(num_y - num.ascent), descent: den_y + den.descent, items }
    }

    fn scripts(&mut self, base: MathBox, sub: Option<&MathNode>, sup: Option<&MathNode>, s: f32, level: u32, italic_base: bool) -> MathBox {
        let ss = self.script(s);
        let sub = sub.map(|n| self.node(n, ss, level + 1));
        let sup = sup.map(|n| self.node(n, ss, level + 1));
        let mut sup_y = -(s * 0.42).max(base.ascent - sup.as_ref().map_or(0.0, |b| b.ascent) * 0.55);
        let mut sub_y =
            (s * 0.2).max(base.descent - sub.as_ref().map_or(0.0, |b| b.descent) * 0.2 + sub.as_ref().map_or(0.0, |b| b.ascent) * 0.25);
        if let (Some(b), Some(p)) = (&sub, &sup) {
            // At least 0.1 em between a subscript and a superscript.
            let clash = (sup_y + p.descent) - (sub_y - b.ascent) + s * 0.1;
            if clash > 0.0 {
                sup_y -= clash / 2.0;
                sub_y += clash / 2.0;
            }
        }
        let x = base.width + s * 0.03;
        // A superscript slightly right of a subscript on an italic letter: its slant.
        let sup_x = x + if italic_base { s * 0.04 } else { 0.0 };
        let mut width = base.width;
        let (mut ascent, mut descent) = (base.ascent, base.descent);
        let mut items = base.items;
        if let Some(b) = &sub {
            items.extend(b.shifted(x, sub_y));
            width = width.max(x + b.width);
            descent = descent.max(sub_y + b.descent);
        }
        if let Some(p) = &sup {
            items.extend(p.shifted(sup_x, sup_y));
            width = width.max(sup_x + p.width);
            ascent = ascent.max(-sup_y + p.ascent);
        }
        MathBox { width: width + s * 0.05, ascent, descent, items }
    }

    fn under_over(
        &mut self,
        base_node: &MathNode,
        under: Option<&MathNode>,
        over: Option<&MathNode>,
        accent_over: bool,
        s: f32,
        level: u32,
    ) -> MathBox {
        let base = self.node(base_node, s, level);
        // The limits of ∑ in a line are at the side, like scripts.
        if is_large(base_node) && !(self.display && level == 0) {
            return self.scripts(base, under, over, s, level, false);
        }
        let mut limits = Vec::new();
        let (mut ascent, mut descent, mut width) = (base.ascent, base.descent, base.width);
        if let Some(over) = over {
            let bar = accent_over && matches!(over, MathNode::Token { text, .. } if BARS.contains(&text.as_str()));
            if bar {
                // A bar over a letter takes its whole width.
                let t = (s * 0.05).max(1.0);
                let y = -(base.ascent + s * 0.08) - t;
                limits.push((None, MathItem::Rule { x: s * 0.04, y, w: base.width - s * 0.04, h: t }));
                ascent = -y;
            } else {
                let size = if accent_over { s } else { self.script(s) };
                let o = self.node(over, size, level + 1);
                let gap = if accent_over { s * 0.02 } else { s * 0.1 };
                let y = -(base.ascent + gap + o.descent);
                width = width.max(o.width);
                ascent = -y + o.ascent;
                limits.extend(o.shifted(0.0, y).into_iter().map(|i| (Some(o.width), i)));
            }
        }
        if let Some(under) = under {
            let size = self.script(s);
            let o = self.node(under, size, level + 1);
            let y = base.descent + s * 0.1 + o.ascent;
            width = width.max(o.width);
            descent = y + o.descent;
            limits.extend(o.shifted(0.0, y).into_iter().map(|i| (Some(o.width), i)));
        }
        // Everything centred in the widest part; a bar stays with its base.
        let dx = (width - base.width) / 2.0;
        let mut items = base.shifted(dx, 0.0);
        items.extend(limits.into_iter().map(|(w, item)| match w {
            Some(w) => item.shifted((width - w) / 2.0, 0.0),
            None => item.shifted(dx, 0.0),
        }));
        MathBox { width, ascent, descent, items }
    }

    fn root(&mut self, radicand: &MathNode, index: Option<&MathNode>, s: f32, level: u32) -> MathBox {
        let body = self.node(radicand, s, level);
        let t = (s * 0.06).max(1.0);
        let gap = s * 0.12;
        let top = -(body.ascent + gap + t / 2.0);
        let bottom = body.descent;
        let surd = s * 0.55;
        let mut shift = 0.0;
        let mut items = Vec::new();
        if let Some(index) = index {
            let i = self.node(index, (s * 0.5).max(self.size * 0.5), level + 2);
            shift = (i.width - surd * 0.5).max(0.0);
            items.extend(i.shifted(0.0, top + (bottom - top) * 0.45 - i.descent - s * 0.05));
        }
        let mid = top + (bottom - top) * 0.62;
        items.push(MathItem::Path {
            points: vec![
                [shift, mid],
                [shift + surd * 0.22, mid - s * 0.08],
                [shift + surd * 0.5, bottom],
                [shift + surd, top],
                [shift + surd + body.width + s * 0.1, top],
            ],
            width: t,
        });
        items.extend(body.shifted(shift + surd + s * 0.05, 0.0));
        MathBox { width: shift + surd + body.width + s * 0.15, ascent: -top + t, descent: bottom + t / 2.0, items }
    }

    fn fenced(&mut self, open: &str, content: MathBox, close: &str, s: f32) -> MathBox {
        let mut side =
            |text: &str| if text.is_empty() { MathBox::default() } else { self.delimiter(text, content.ascent, content.descent, s) };
        let (left, right) = (side(open), side(close));
        let pad = s * 0.05;
        let mut items = left.items.clone();
        items.extend(content.shifted(left.width + pad, 0.0));
        items.extend(right.shifted(left.width + pad + content.width + pad, 0.0));
        MathBox {
            width: left.width + content.width + right.width + 2.0 * pad,
            ascent: left.ascent.max(content.ascent).max(right.ascent),
            descent: left.descent.max(content.descent).max(right.descent),
            items,
        }
    }

    /// A fence as high as its contents, around the axis: a low one is a glyph, a high one a polyline.
    fn delimiter(&mut self, text: &str, ascent: f32, descent: f32, s: f32) -> MathBox {
        let a = s * 0.25;
        let half = (ascent - a).max(descent + a) + s * 0.08;
        if 2.0 * half <= s * 1.25 {
            let (width, ascent, descent) = self.metrics.measure(text, s, false, false);
            return MathBox {
                width,
                ascent,
                descent,
                items: vec![MathItem::Text { text: text.to_owned(), x: 0.0, y: 0.0, size: s, italic: false, bold: false }],
            };
        }
        let (top, bottom) = (-a - half, -a + half);
        let h = bottom - top;
        let t = (s * 0.055).max(1.0);
        let w = s * 0.3 + (h * 0.04).min(s * 0.2);
        let points: Vec<[f32; 2]> = match text {
            "(" | ")" => {
                // An arc of an ellipse: "(" bulges to the left.
                let (dir, cx) = if text == "(" { (-1.0, w * 0.85) } else { (1.0, w * 0.15) };
                (0..=12)
                    .map(|i| {
                        let angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / 12.0;
                        [cx + dir * (w * 0.7) * angle.cos(), -a + (h / 2.0) * angle.sin()]
                    })
                    .collect()
            }
            "[" | "]" => {
                let (inner, outer) = if text == "[" { (w * 0.8, w * 0.25) } else { (w * 0.2, w * 0.75) };
                vec![[inner, top], [outer, top], [outer, bottom], [inner, bottom]]
            }
            "{" | "}" => {
                let (tip, body, edge) = if text == "{" { (w * 0.1, w * 0.5, w * 0.85) } else { (w * 0.9, w * 0.5, w * 0.15) };
                vec![
                    [edge, top],
                    [body, top + h * 0.08],
                    [body, -a - h * 0.06],
                    [tip, -a],
                    [body, -a + h * 0.06],
                    [body, bottom - h * 0.08],
                    [edge, bottom],
                ]
            }
            "⟨" | "〈" => vec![[w * 0.8, top], [w * 0.2, -a], [w * 0.8, bottom]],
            "⟩" | "〉" => vec![[w * 0.2, top], [w * 0.8, -a], [w * 0.2, bottom]],
            "‖" => {
                return MathBox {
                    width: w,
                    ascent: -top,
                    descent: bottom,
                    items: vec![MathItem::Rule { x: w * 0.3, y: top, w: t, h }, MathItem::Rule { x: w * 0.65, y: top, w: t, h }],
                };
            }
            _ => {
                // | ⌈ ⌉ ⌊ ⌋: a bar.
                return MathBox {
                    width: w,
                    ascent: -top,
                    descent: bottom,
                    items: vec![MathItem::Rule { x: w / 2.0 - t / 2.0, y: top, w: t, h }],
                };
            }
        };
        MathBox { width: w, ascent: -top, descent: bottom, items: vec![MathItem::Path { points, width: t }] }
    }

    fn matrix(&mut self, rows: &[Vec<MathNode>], align: &[ColumnAlign], lines: &[bool], s: f32, level: u32) -> MathBox {
        if rows.is_empty() {
            return MathBox::default();
        }
        let cells: Vec<Vec<MathBox>> = rows.iter().map(|row| row.iter().map(|c| self.node(c, s, level)).collect()).collect();
        let columns = cells.iter().map(Vec::len).max().unwrap_or(0);
        let widths: Vec<f32> = (0..columns).map(|c| cells.iter().filter_map(|r| r.get(c)).map(|b| b.width).fold(0.0, f32::max)).collect();
        let col_gap = s * 0.8;
        // The limits of ∑ in a display formula stand above and below: a chain of equalities needs more air.
        let row_gap = s * if self.display && level == 0 { 0.4 } else { 0.25 };
        let extent = |row: &Vec<MathBox>, f: fn(&MathBox) -> f32, empty: f32| {
            if row.is_empty() { empty } else { row.iter().map(f).fold(0.0, f32::max) }
        };
        let ascents: Vec<f32> = cells.iter().map(|r| extent(r, |b| b.ascent, s * 0.7)).collect();
        let descents: Vec<f32> = cells.iter().map(|r| extent(r, |b| b.descent, s * 0.2)).collect();
        let total = ascents.iter().sum::<f32>() + descents.iter().sum::<f32>() + row_gap * (cells.len() - 1) as f32;
        // The matrix's middle is on the axis.
        let top = -s * 0.25 - total / 2.0;
        let mut items = Vec::new();
        let mut y = top;
        for (r, row) in cells.iter().enumerate() {
            y += ascents[r];
            let mut x = 0.0;
            for (c, width) in widths.iter().enumerate() {
                if let Some(cell) = row.get(c) {
                    let dx = match align.get(c).or(align.last()).copied().unwrap_or(ColumnAlign::Center) {
                        ColumnAlign::Left => 0.0,
                        ColumnAlign::Center => (width - cell.width) / 2.0,
                        ColumnAlign::Right => width - cell.width,
                    };
                    items.extend(cell.shifted(x + dx, y));
                }
                x += width + col_gap;
            }
            y += descents[r] + row_gap;
        }
        // Lines between columns (`columnlines`): in the middle of the gap, the whole height.
        let t = (s * 0.05).max(1.0);
        let mut x = 0.0;
        for (c, width) in widths.iter().enumerate().take(columns.saturating_sub(1)) {
            x += width;
            if lines.get(c).copied().unwrap_or(lines.len() == 1 && lines[0]) {
                items.push(MathItem::Rule { x: x + col_gap / 2.0 - t / 2.0, y: top, w: t, h: total });
            }
            x += col_gap;
        }
        let width = widths.iter().sum::<f32>() + col_gap * columns.saturating_sub(1) as f32;
        MathBox { width, ascent: -top, descent: top + total, items }
    }
}

/// Whether `mover` with this mark is an accent even without `accent="true"`.
pub(crate) fn is_accent_mark(text: &str) -> bool {
    BARS.contains(&text) || (!text.is_empty() && text.chars().all(|c| ACCENT_MARKS.contains(c)))
}

/// The spaces before and after part `i` of a row: a binary operator 0.22 em, a relation 0.28 em; none at a unary one
/// (first, or after an operator).
fn spacing(children: &[MathNode], i: usize, s: f32) -> (f32, f32) {
    let MathNode::Token { text, kind: TokenKind::Operator, .. } = &children[i] else { return (0.0, 0.0) };
    let prev = i.checked_sub(1).map(|p| &children[p]);
    let unary = match prev {
        None => true,
        Some(MathNode::Token { text: p, kind, .. }) => {
            (*kind == TokenKind::Operator && operator_class(p) != OperatorClass::Fence) || OPENING.contains(&p.as_str())
        }
        _ => false,
    };
    match operator_class(text) {
        OperatorClass::Binary if unary => (0.0, 0.0),
        OperatorClass::Binary => (s * 0.22, s * 0.22),
        OperatorClass::Relation if prev.is_none() => (0.0, s * 0.28),
        OperatorClass::Relation => (s * 0.28, s * 0.28),
        OperatorClass::Punctuation => (0.0, if text == "," || text == ";" { s * 0.17 } else { 0.0 }),
        OperatorClass::Large => (0.0, s * 0.1),
        // det, dim, lim
        OperatorClass::Ordinary if text.chars().count() > 1 => (s * 0.1, s * 0.1),
        _ => (0.0, 0.0),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::{ColumnAlign, MathNode as N, TokenKind};

    /// A sign is 0.5 em wide, 0.7 em above the baseline, 0.2 em below.
    struct Fixed;
    impl MathMetrics for Fixed {
        fn measure(&mut self, text: &str, size: f32, _: bool, _: bool) -> (f32, f32, f32) {
            (text.chars().count() as f32 * size * 0.5, size * 0.7, size * 0.2)
        }
    }

    fn set(n: &N) -> MathBox {
        MathLayout::new(&mut Fixed, 20.0, false).layout(n)
    }
    fn set_display(n: &N) -> MathBox {
        MathLayout::new(&mut Fixed, 20.0, true).layout(n)
    }
    fn mi(s: &str) -> N {
        N::token(s, TokenKind::Identifier)
    }
    fn mn(s: &str) -> N {
        N::token(s, TokenKind::Number)
    }
    fn mo(s: &str) -> N {
        N::token(s, TokenKind::Operator)
    }
    /// (text, x, y, size, italic) of the box's texts.
    fn texts(b: &MathBox) -> Vec<(String, f32, f32, f32, bool)> {
        b.items
            .iter()
            .filter_map(|i| match i {
                MathItem::Text { text, x, y, size, italic, .. } => Some((text.clone(), *x, *y, *size, *italic)),
                _ => None,
            })
            .collect()
    }
    fn rules(b: &MathBox) -> Vec<(f32, f32)> {
        b.items.iter().filter_map(|i| if let MathItem::Rule { y, h, .. } = i { Some((*y, *h)) } else { None }).collect()
    }
    fn paths(b: &MathBox) -> usize {
        b.items.iter().filter(|i| matches!(i, MathItem::Path { .. })).count()
    }
    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn an_identifier_is_in_italics_and_a_number_upright() {
        let g = texts(&set(&N::Row(vec![mi("x"), mi("det"), mn("2")])));
        assert_eq!(g.iter().map(|t| t.4).collect::<Vec<_>>(), [true, false, false]);
    }

    #[test]
    fn a_binary_operator_has_spaces_and_a_unary_minus_none() {
        let g = texts(&set(&N::Row(vec![mi("a"), mo("+"), mi("b")])));
        assert!(close(g[1].1, 10.0 + 0.8 + 4.4), "0.22 em before the plus: {}", g[1].1);
        let neg = texts(&set(&N::Row(vec![mo("−"), mi("a")])));
        assert!(close(neg[1].1, 10.0), "a unary minus has no space");
    }

    #[test]
    fn a_subscript_is_smaller_and_below() {
        let b = set(&N::Scripts { base: mi("a").into(), sub: Some(mn("11").into()), sup: None });
        let g = texts(&b);
        assert!(close(g[1].3, 20.0 * 0.71));
        assert!(g[1].2 > 0.0, "below the baseline");
        assert!(g[1].1 >= g[0].1 + 10.0);
        assert!(b.descent > 20.0 * 0.2);
    }

    #[test]
    fn a_superscript_is_above() {
        let g = texts(&set(&N::Scripts { base: mi("x").into(), sub: None, sup: Some(mn("2").into()) }));
        assert!(g[1].2 < -20.0 * 0.4);
    }

    #[test]
    fn a_fraction_sits_around_the_axis() {
        let b = set(&N::Frac { num: mi("a").into(), den: mi("b").into(), line: true });
        let g = texts(&b);
        let rule = rules(&b)[0];
        assert!(rule.0 < -4.0 && rule.0 > -6.0, "the rule on the axis");
        assert!(g[0].2 < rule.0 && g[1].2 > rule.0);
        assert!(close(g[0].3, 20.0 * 0.71), "its parts are smaller");
    }

    #[test]
    fn a_sums_limits_are_at_the_side_in_a_line_and_above_and_below_in_a_display() {
        let sum = N::UnderOver {
            base: mo("∑").into(),
            under: Some(N::Row(vec![mi("i")]).into()),
            over: Some(mi("n").into()),
            accent_over: false,
        };
        let g = texts(&set(&sum));
        assert!(g[1..].iter().all(|t| t.1 >= g[0].1 + 10.0), "at the side");
        let d = texts(&set_display(&sum));
        assert!(d.iter().find(|t| t.0 == "i").unwrap().2 > 0.0 && d.iter().find(|t| t.0 == "n").unwrap().2 < 0.0, "below and above");
        assert!(close(d[0].3, 28.0), "larger in a display");
    }

    #[test]
    fn a_bar_is_a_rule_over_its_base() {
        let b = set(&N::UnderOver { base: mi("x").into(), under: None, over: Some(mo("¯").into()), accent_over: true });
        assert!(rules(&b)[0].0 < -14.0);
        assert_eq!(texts(&b).len(), 1);
    }

    #[test]
    fn a_root_has_a_sign_and_a_bar() {
        let b = set(&N::Root { radicand: mi("x").into(), index: None });
        assert_eq!(paths(&b), 1);
        assert!(b.ascent > 14.0);
    }

    #[test]
    fn a_matrix_stands_in_stretched_brackets() {
        let matrix = N::Matrix {
            rows: vec![vec![mn("1"), mn("22")], vec![mn("333"), mn("4")]],
            align: vec![ColumnAlign::Right, ColumnAlign::Left],
            lines: vec![true],
        };
        let b = set(&N::Fenced { open: "[".into(), close: "]".into(), content: matrix.into() });
        assert_eq!(paths(&b), 2, "tall brackets are polylines");
        let g = texts(&b);
        let (one, three) = (g.iter().find(|t| t.0 == "1").unwrap(), g.iter().find(|t| t.0 == "333").unwrap());
        assert!(close(three.1 + 30.0, one.1 + 10.0), "right edges together");
        assert!(three.2 > one.2, "rows one under another");
        assert_eq!(rules(&b).len(), 1, "the column's line");
        assert!((-7.0..=-3.0).contains(&((b.descent - b.ascent) / 2.0)), "the middle on the axis");
    }

    #[test]
    fn low_parentheses_are_glyphs() {
        let b = set(&N::Fenced { open: "(".into(), close: ")".into(), content: N::Row(vec![mi("x"), mo(","), mi("y")]).into() });
        assert_eq!(paths(&b), 0);
        assert_eq!(texts(&b).iter().map(|t| t.0.as_str()).collect::<Vec<_>>(), ["(", "x", ",", "y", ")"]);
    }

    #[test]
    fn an_empty_row_is_empty() {
        assert_eq!(set(&N::Row(Vec::new())).width, 0.0);
    }
}
