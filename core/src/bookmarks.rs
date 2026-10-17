//! Bookmarks: places a reader marks, kept as Locators (Android's `toggleBookmark`). A page (or a screen) is marked when
//! a bookmark lies in its range `[from, to)`.

use crate::model::{Block, Book, Locator};

/// Characters of text shown for a bookmark in the list.
const SNIPPET: usize = 160;

/// Sets a bookmark at `from` when none lies in `[from, to)`, else removes those that do; keeps `marks` in reading
/// order. True when it set one.
pub fn toggle(marks: &mut Vec<Locator>, from: Locator, to: Locator) -> bool {
    let before = marks.len();
    marks.retain(|m| !(from <= *m && *m < to));
    if marks.len() < before {
        return false;
    }
    let i = marks.partition_point(|m| *m < from);
    marks.insert(i, from);
    true
}

/// Whether a bookmark lies in `[from, to)`.
pub fn marked(marks: &[Locator], from: Locator, to: Locator) -> bool {
    marks.iter().any(|m| from <= *m && *m < to)
}

/// A bookmark as the list shows it: the section it is in, its share of the book before it and the text from it.
#[derive(Debug, Clone, PartialEq)]
pub struct Entry {
    pub at: Locator,
    pub section: Option<String>,
    pub position: f32,
    pub snippet: String,
}

/// The list entries of `marks`.
pub fn entries(book: &Book, marks: &[Locator]) -> Vec<Entry> {
    let contents = book.contents();
    let positions = crate::state::progress_of(book, marks);
    let end = Locator { chapter: book.chapters.len(), ..Locator::default() };
    marks
        .iter()
        .zip(positions)
        .map(|(&at, position)| Entry {
            at,
            section: contents.iter().rev().find(|e| e.target <= at).map(|e| e.title.clone()),
            position,
            snippet: book.text(after_headings(book, at), end, SNIPPET).split_whitespace().collect::<Vec<_>>().join(" "),
        })
        .collect()
}

/// `at`, or the start of the first block after the headings it is in or before (the list shows the section already).
fn after_headings(book: &Book, at: Locator) -> Locator {
    let Some(chapter) = book.chapters.get(at.chapter) else { return at };
    let skip = chapter.blocks[at.block.min(chapter.blocks.len())..]
        .iter()
        .take_while(|b| matches!(b, Block::Heading { .. } | Block::EmptyLine))
        .count();
    if skip == 0 { at } else { Locator { block: at.block + skip, offset: 0, ..at } }
}

#[cfg(test)]
mod tests;
