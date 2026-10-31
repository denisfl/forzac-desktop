//! Search in a book: the Android app's rules (`Search.kt`). Case and "ё"/"е" do not matter, a
//! soft hyphen in the text does not interrupt a match, a non-breaking space equals a space and a space in the query
//! matches any run of white space. Paragraphs, headings and table cells are searched (a match stays in its cell); notes
//! are not.

use std::ops::Range;

use crate::math::PLACEHOLDER;
use crate::model::{Block, Book, Locator, StyledText};

/// A match: `[start, end)` in the book's text, a snippet around it with the match at `matched` (bytes of `snippet`),
/// and where it is: the section (the contents entry at or before it) and the share of the book before it.
#[derive(Debug, Clone, PartialEq)]
pub struct Hit {
    pub start: Locator,
    pub end: Locator,
    pub snippet: String,
    pub matched: Range<usize>,
    pub section: Option<String>,
    pub position: f32,
}

const MIN_QUERY: usize = 2;
/// Characters of context before and after a match in its snippet.
const BEFORE: usize = 40;
const AFTER: usize = 80;
const SOFT_HYPHEN: char = '\u{AD}';

/// A character as search compares it: lower case, "ё" as "е", a non-breaking space as a space.
fn normal(c: char) -> char {
    match c {
        'ё' | 'Ё' => 'е',
        '\u{A0}' | '\u{202F}' => ' ',
        c => c.to_lowercase().next().unwrap_or(c),
    }
}

/// The query as compared, or `None` when it is too short to search.
fn query(q: &str) -> Option<Vec<char>> {
    let q: Vec<char> = q.chars().filter(|&c| c != SOFT_HYPHEN).map(normal).collect();
    let trimmed = q.iter().collect::<String>();
    (trimmed.trim().chars().count() >= MIN_QUERY).then_some(q)
}

/// The matches of `q` in the book, in reading order.
pub fn search(book: &Book, q: &str) -> Vec<Hit> {
    let Some(q) = query(q) else { return Vec::new() };
    let contents = book.contents();
    let mut hits = Vec::new();
    // Text before each block, for the position.
    let total: usize = book.chapters.iter().flat_map(|c| &c.blocks).map(Block::text_len).sum();
    let mut before = 0;
    // One buffer for every block's characters: a book has tens of thousands of blocks.
    let mut chars: Vec<(usize, char)> = Vec::new();
    for (c, chapter) in book.chapters.iter().enumerate() {
        for (b, block) in chapter.blocks.iter().enumerate() {
            // The texts to search and where each starts in the block's text: a table row's cells one by one.
            let texts: Vec<(&StyledText, usize)> = match block {
                Block::Paragraph { text, .. } | Block::Heading { text, .. } => vec![(text, 0)],
                Block::TableRow { table, row } => table.rows[*row].iter().map(|c| &c.text).zip(table.cell_starts(*row)).collect(),
                _ => Vec::new(),
            };
            for (styled, base) in texts {
                let text = styled.text.as_str();
                chars.clear();
                chars.extend(text.char_indices());
                let mut i = 0;
                while i < chars.len() {
                    match match_at(&chars, i, &q) {
                        Some(end) => {
                            let (start, end_byte) = (base + chars[i].0, base + chars.get(end).map_or(text.len(), |c| c.0));
                            let at = Locator { chapter: c, block: b, offset: start };
                            let section = contents.iter().rev().find(|e| e.target <= at).map(|e| e.title.clone());
                            let position = if total == 0 { 0.0 } else { (before + start.min(block.text_len())) as f32 / total as f32 };
                            let (snippet, matched) = snippet(&chars, i, end, &|at| book.formula_at(styled, at).map(|f| f.text.as_str()));
                            hits.push(Hit { start: at, end: Locator { offset: end_byte, ..at }, snippet, matched, section, position });
                            i = end.max(i + 1);
                        }
                        None => i += 1,
                    }
                }
            }
            before += block.text_len();
        }
    }
    hits
}

/// A match in a PDF page's text: the snippet, the match in it, and where it is in the page's text in UTF-16 units.
pub struct PageHit {
    pub snippet: String,
    pub matched: Range<usize>,
    pub place: Range<usize>,
}

/// A PDF page's text as it is searched, with each character's UTF-16 offset in `text`: a hyphen at a line's end joins
/// the word, and line breaks are spaces. PDFium marks a hyphen it took for a line's end with U+FFFE or U+0002.
fn page_text(text: &str) -> (String, Vec<usize>) {
    let mut at = 0;
    let raw: Vec<(char, usize)> = text
        .chars()
        .map(|c| {
            at += c.len_utf16();
            (c, at - c.len_utf16())
        })
        .collect();
    let line_break = |i: usize| match raw.get(i).map(|r| r.0) {
        Some('\n') => 1,
        Some('\r') if raw.get(i + 1).is_some_and(|r| r.0 == '\n') => 2,
        _ => 0,
    };
    let (mut out, mut offsets) = (String::with_capacity(text.len()), Vec::with_capacity(raw.len()));
    let mut i = 0;
    while i < raw.len() {
        let (c, at) = raw[i];
        let marked = c == '\u{FFFE}' || c == '\u{2}';
        if marked || (c == '-' && line_break(i + 1) > 0) {
            i += 1;
            for _ in 0..if marked { 1 } else { 2 } {
                i += line_break(i);
            }
            continue;
        }
        let breaks = line_break(i);
        out.push(if breaks > 0 || c == '\r' { ' ' } else { c });
        offsets.push(at);
        i += breaks.max(1);
    }
    (out, offsets)
}

/// The matches of `q` in a PDF page's text.
pub fn search_page(text: &str, q: &str) -> Vec<PageHit> {
    let Some(q) = query(q) else { return Vec::new() };
    let (text, offsets) = page_text(text);
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let (mut hits, mut i) = (Vec::new(), 0);
    while i < chars.len() {
        match match_at(&chars, i, &q) {
            Some(end) => {
                let (snippet, matched) = snippet(&chars, i, end, &|_| None);
                let place = offsets[i]..offsets[end - 1] + chars[end - 1].1.len_utf16();
                hits.push(PageHit { snippet, matched, place });
                i = end.max(i + 1);
            }
            None => i += 1,
        }
    }
    hits
}

/// Where a match of `q` starting at character `from` ends (a character index), if there is one.
fn match_at(text: &[(usize, char)], from: usize, q: &[char]) -> Option<usize> {
    if normal(text[from].1) != q[0] {
        return None;
    }
    let (mut t, mut k) = (from, 0);
    while k < q.len() {
        let &(_, c) = text.get(t)?;
        if c == SOFT_HYPHEN || c == '\r' {
            t += 1;
        } else if q[k] == ' ' && c.is_whitespace() {
            while text.get(t).is_some_and(|(_, c)| c.is_whitespace()) {
                t += 1;
            }
            k += 1;
        } else if normal(c) != q[k] {
            return None;
        } else {
            t += 1;
            k += 1;
        }
    }
    Some(t)
}

/// The snippet around characters `[start, end)`: up to `BEFORE` characters before and `AFTER` after, cut at spaces,
/// "…" where cut; soft hyphens dropped and line breaks as spaces. Returns it and the match's byte range in it.
fn snippet<'a>(text: &[(usize, char)], start: usize, end: usize, formula: &dyn Fn(usize) -> Option<&'a str>) -> (String, Range<usize>) {
    let mut from = start.saturating_sub(BEFORE);
    if from > 0
        && let Some(space) = (from..start).find(|&i| text[i].1 == ' ')
    {
        from = space + 1;
    }
    let mut to = (end + AFTER).min(text.len());
    if to < text.len()
        && let Some(space) = (end + 1..to).rev().find(|&i| text[i].1 == ' ')
    {
        to = space;
    }
    let clean = |a: usize, b: usize| -> String {
        let mut out = String::new();
        for &(at, c) in &text[a..b] {
            match c {
                SOFT_HYPHEN | '\r' => {}
                '\n' => out.push(' '),
                PLACEHOLDER => out.push_str(formula(at).unwrap_or("")),
                c => out.push(c),
            }
        }
        out
    };
    let mut out = String::new();
    if from > 0 {
        out.push('…');
    }
    out.push_str(&clean(from, start));
    let m0 = out.len();
    out.push_str(&clean(start, end));
    let m1 = out.len();
    out.push_str(&clean(end, to));
    if to < text.len() {
        out.push('…');
    }
    (out, m0..m1)
}

#[cfg(test)]
mod tests;
