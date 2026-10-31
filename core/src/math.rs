//! MathML formulas (`math`): a formula's tree and its linear text, a port of Android's `model/Math.kt`. In a paragraph's
//! text a formula is one character, [`PLACEHOLDER`], with [`crate::model::Style::Math`] whose target is its key in
//! [`crate::model::Book::math`]; a display formula is a block of its own. Positions, selection and search therefore count
//! it as one character. The linear text stands in wherever text leaves the page: quotes, copying, search results, MOBI.

/// A formula's character in a paragraph's text.
pub const PLACEHOLDER: char = '\u{FFFC}';

/// A formula: its tree, whether it is a display formula, its linear text.
#[derive(Debug, Clone, PartialEq)]
pub struct Formula {
    pub node: MathNode,
    pub display: bool,
    pub text: String,
}

impl Formula {
    pub fn new(node: MathNode, display: bool) -> Self {
        let text = node.linear().trim().to_owned();
        Formula { node, display, text }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum MathNode {
    /// `mi`, `mn`, `mo`, `mtext`; `scale` is `mathsize` against the formula's size.
    Token {
        text: String,
        kind: TokenKind,
        variant: Variant,
        scale: f32,
    },
    Row(Vec<MathNode>),
    Frac {
        num: Box<MathNode>,
        den: Box<MathNode>,
        line: bool,
    },
    /// `msub`, `msup`, `msubsup`.
    Scripts {
        base: Box<MathNode>,
        sub: Option<Box<MathNode>>,
        sup: Option<Box<MathNode>>,
    },
    /// `munder`, `mover`, `munderover`; an accent is a mark over the base, not a limit.
    UnderOver {
        base: Box<MathNode>,
        under: Option<Box<MathNode>>,
        over: Option<Box<MathNode>>,
        accent_over: bool,
    },
    /// `msqrt` (no index) and `mroot`.
    Root {
        radicand: Box<MathNode>,
        index: Option<Box<MathNode>>,
    },
    /// `mfenced`, its separators already in the content.
    Fenced {
        open: String,
        close: String,
        content: Box<MathNode>,
    },
    /// `mtable`: rows of cells, the columns' alignment and the lines between columns.
    Matrix {
        rows: Vec<Vec<MathNode>>,
        align: Vec<ColumnAlign>,
        lines: Vec<bool>,
    },
    /// `mspace`: its width in em.
    Space(f32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Identifier,
    Number,
    Operator,
    Text,
}

/// `mathvariant`: by default a one-letter variable is in italics, the rest upright.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variant {
    Default,
    Normal,
    Bold,
    Italic,
    BoldItalic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnAlign {
    Left,
    Center,
    Right,
}

/// An operator's class: the spaces around it, in typesetting and in the linear text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatorClass {
    Binary,
    Relation,
    Punctuation,
    Large,
    Fence,
    Ordinary,
}

pub fn operator_class(text: &str) -> OperatorClass {
    match text {
        "+" | "−" | "-" | "±" | "∓" | "×" | "÷" | "∗" | "·" | "⋅" | "∘" | "⊕" | "⊗" | "⊖" | "∪" | "∩" | "∖" | "∧" | "∨" | "⊠" | "△" => {
            OperatorClass::Binary
        }
        "=" | "≠" | "<" | ">" | "≤" | "≥" | "≈" | "≡" | "≅" | "∼" | "∝" | "∈" | "∉" | "∋" | "⊂" | "⊆" | "⊃" | "⊇" | "⊄" | "→" | "←"
        | "↔" | "⇒" | "⇐" | "⇔" | "↦" | "≔" | ":=" | "≪" | "≫" | "⊥" | "∥" | "|=" | "⊢" | "⊨" => {
            OperatorClass::Relation
        }
        "," | ";" | "." | ":" => OperatorClass::Punctuation,
        "∑" | "∏" | "∐" | "∫" | "∬" | "∭" | "∮" | "⋃" | "⋂" | "⨁" | "⨂" => OperatorClass::Large,
        "(" | ")" | "[" | "]" | "{" | "}" | "|" | "‖" | "⟨" | "⟩" | "⌈" | "⌉" | "⌊" | "⌋" => OperatorClass::Fence,
        _ => OperatorClass::Ordinary,
    }
}

/// A bar over a letter: `mover` with one of these is an accent (a mean, a conjugate).
pub const BARS: &[&str] = &["¯", "‾", "―", "_", "-", "−", "\u{304}", "\u{305}"];

/// Marks that make `mover` an accent even without `accent="true"`.
pub const ACCENT_MARKS: &str = "^ˆ˜~→⃗˙¨";

impl MathNode {
    pub fn token(text: &str, kind: TokenKind) -> MathNode {
        MathNode::Token { text: text.to_owned(), kind, variant: Variant::Default, scale: 1.0 }
    }

    fn is_large(&self) -> bool {
        matches!(self, MathNode::Token { text, kind: TokenKind::Operator, .. } if operator_class(text) == OperatorClass::Large)
    }

    /// The linear text: `x_i`, `x^2`, `a/b`, `√(x)`, `[a b; c d]`, spaces around binary operators and relations.
    pub fn linear(&self) -> String {
        match self {
            MathNode::Token { text, kind: TokenKind::Operator, .. } => match operator_class(text) {
                OperatorClass::Binary | OperatorClass::Relation => format!(" {text} "),
                OperatorClass::Punctuation => format!("{text} "),
                _ => text.clone(),
            },
            MathNode::Token { text, .. } => text.clone(),
            MathNode::Row(children) => {
                let mut out = String::new();
                for (i, child) in children.iter().enumerate() {
                    // A unary minus (first, or after an operator) without spaces: "−1", not "− 1".
                    let after_operator = i == 0 || matches!(children[i - 1], MathNode::Token { kind: TokenKind::Operator, .. });
                    match child {
                        MathNode::Token { text, kind: TokenKind::Operator, .. }
                            if after_operator && operator_class(text) == OperatorClass::Binary =>
                        {
                            out.push_str(text)
                        }
                        _ => out.push_str(&child.linear()),
                    }
                }
                let mut single = String::with_capacity(out.len());
                for c in out.chars() {
                    if c != ' ' || !single.ends_with(' ') {
                        single.push(c);
                    }
                }
                single
            }
            MathNode::Frac { num, den, .. } => format!("{}/{}", num.group(false), den.group(false)),
            MathNode::Scripts { base, sub, sup } => limits(base, sub, sup),
            MathNode::UnderOver { base, under, over, accent_over } => {
                let b = base.linear().trim().to_owned();
                match over.as_deref() {
                    Some(MathNode::Token { text, .. }) if *accent_over && BARS.contains(&text.as_str()) => b + "\u{304}",
                    Some(o) if *accent_over => b + o.linear().trim(),
                    _ => limits(base, under, over),
                }
            }
            MathNode::Root { radicand, index } => {
                let index = index.as_ref().map_or(String::new(), |i| format!("[{}]", i.linear().trim()));
                format!("{index}√{}", radicand.group(true))
            }
            MathNode::Fenced { open, close, content } => format!("{open}{}{close}", content.linear().trim()),
            MathNode::Matrix { rows, .. } => rows
                .iter()
                .map(|row| row.iter().map(|c| c.linear().trim().to_owned()).collect::<Vec<_>>().join(" "))
                .collect::<Vec<_>>()
                .join("; "),
            MathNode::Space(em) => if *em >= 0.2 { " " } else { "" }.to_owned(),
        }
    }

    /// A part of the linear text: one "word" as it is, anything else in parentheses.
    fn group(&self, always: bool) -> String {
        let text = self.linear().trim().to_owned();
        let simple = !always
            && (matches!(self, MathNode::Token { .. } | MathNode::Fenced { .. })
                || (!text.contains(' ') && !matches!(self, MathNode::Row(_))));
        if simple || (text.chars().count() == 1 && !always) { text } else { format!("({text})") }
    }
}

/// `∑_(i = 1)^n`: a large operator with its limits is a "word" of its own.
fn limits(base: &MathNode, below: &Option<Box<MathNode>>, above: &Option<Box<MathNode>>) -> String {
    let mut out = base.linear().trim().to_owned();
    if let Some(b) = below {
        out.push('_');
        out.push_str(&b.group(false));
    }
    if let Some(a) = above {
        out.push('^');
        out.push_str(&a.group(false));
    }
    if base.is_large() {
        out.push(' ');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mi(s: &str) -> MathNode {
        MathNode::token(s, TokenKind::Identifier)
    }
    fn mn(s: &str) -> MathNode {
        MathNode::token(s, TokenKind::Number)
    }
    fn mo(s: &str) -> MathNode {
        MathNode::token(s, TokenKind::Operator)
    }
    fn sub(b: MathNode, s: MathNode) -> MathNode {
        MathNode::Scripts { base: b.into(), sub: Some(s.into()), sup: None }
    }
    fn sup(b: MathNode, s: MathNode) -> MathNode {
        MathNode::Scripts { base: b.into(), sub: None, sup: Some(s.into()) }
    }
    fn text(n: MathNode) -> String {
        Formula::new(n, false).text
    }

    #[test]
    fn the_linear_text_is_androids() {
        assert_eq!(text(sub(mi("a"), mn("11"))), "a_11");
        assert_eq!(text(MathNode::Row(vec![sup(mi("x"), mn("2")), mo("+"), sup(mi("y"), mn("2"))])), "x^2 + y^2");
        let frac = MathNode::Frac { num: MathNode::Row(vec![mi("a"), mo("+"), mi("b")]).into(), den: mi("c").into(), line: true };
        assert_eq!(text(frac), "(a + b)/c");
        assert_eq!(text(MathNode::Root { radicand: mi("x").into(), index: None }), "√(x)");
        let matrix = MathNode::Matrix { rows: vec![vec![mn("1"), mn("2")], vec![mn("3"), mn("4")]], align: vec![], lines: vec![] };
        let fenced = MathNode::Fenced { open: "[".into(), close: "]".into(), content: matrix.into() };
        assert_eq!(text(MathNode::Row(vec![mi("A"), mo("="), fenced])), "A = [1 2; 3 4]");
        let sum = MathNode::UnderOver {
            base: mo("∑").into(),
            under: Some(MathNode::Row(vec![mi("i"), mo("="), mn("1")]).into()),
            over: Some(mi("n").into()),
            accent_over: false,
        };
        assert_eq!(text(sum), "∑_(i = 1)^n");
        let bar = MathNode::UnderOver { base: mi("x").into(), under: None, over: Some(mo("¯").into()), accent_over: true };
        assert_eq!(text(bar), "x\u{304}", "a bar over a letter is a combining mark");
        assert_eq!(text(sup(mi("E"), MathNode::Row(vec![mo("−"), mn("1")]))), "E^(−1)", "a unary minus");
        assert_eq!(text(MathNode::Row(vec![mi("a"), mo("="), mo("−"), mi("b")])), "a = −b");
        let f = MathNode::Fenced { open: "(".into(), close: ")".into(), content: MathNode::Row(vec![mi("x"), mo(","), mi("y")]).into() };
        assert_eq!(text(MathNode::Row(vec![mi("f"), f])), "f(x, y)");
    }

    #[test]
    fn operators_have_classes() {
        assert_eq!(operator_class("≤"), OperatorClass::Relation);
        assert_eq!(operator_class("−"), OperatorClass::Binary);
        assert_eq!(operator_class("∑"), OperatorClass::Large);
        assert_eq!(operator_class("det"), OperatorClass::Ordinary);
    }
}
