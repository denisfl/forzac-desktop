use std::sync::Arc;

use super::*;
use crate::hyphen::Hyphenator;
use crate::layout::{PageStyle, Typesetter, paginate};
use crate::model::{Block, Book, Chapter, Kind, StyledText, Table, TableCell};

const TEXT: &str =
    "Слезы навернулись у нее на глаза. Видно, ей нравилась эта распущенность, видно, ее рассердило, что я хотел пристыдить ее.";

fn paragraph(text: &str) -> Block {
    Block::Paragraph { text: StyledText { text: text.into(), styles: vec![] }, kind: Kind::Body }
}

fn book(blocks: Vec<Block>) -> Book {
    Book { language: Some("ru".into()), chapters: vec![Chapter { title: None, blocks }], ..Book::default() }
}

/// Words of the first page of `book` set in a column `width` wide.
fn words(book: &Book, width: f32) -> Vec<Word> {
    crate::fonts::set_assets(std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets")));
    let style = PageStyle::for_window(width, 900.0, 20.0);
    let mut t = Typesetter::new(style, Hyphenator::for_language(Some("ru")).map(Arc::new));
    paginate(book, 0, &style, &mut t).words(0, &style, book)
}

fn text<'a>(book: &'a Book, w: &Word) -> &'a str {
    &book.chapters[0].blocks[w.start.block].text().unwrap().text[w.start.offset..w.end.offset]
}

#[test]
fn words_carry_their_punctuation_and_hyphenated_words_one_range() {
    let b = book(vec![paragraph(TEXT), paragraph(TEXT)]);
    let all = words(&b, 300.0);
    let mut first: Vec<&str> = all.iter().map(|w| text(&b, w)).collect();
    first.dedup(); // a hyphenated word's pieces
    first.truncate(4);
    assert_eq!(first, ["Слезы", "навернулись", "у", "нее"]);
    assert!(all.iter().any(|w| text(&b, w) == "глаза."), "punctuation stays with its word");
    // A narrow column hyphenates: two pieces on two lines with one range.
    let split = all.windows(2).find(|p| p[0].start == p[1].start).expect("a hyphenated word");
    assert!(split[1].rect.y > split[0].rect.y);
    assert!(!text(&b, &split[0]).contains(' '));
    assert_eq!(all.iter().filter(|w| w.start.block == 1).count(), all.iter().filter(|w| w.start.block == 0).count());
}

#[test]
fn moving_by_words_and_lines() {
    let b = book(vec![paragraph(TEXT)]);
    let all = words(&b, 300.0);
    let split = (0..all.len() - 1).find(|&i| all[i].start == all[i + 1].start).unwrap();
    // The next word after a hyphenated word's first piece skips its second piece.
    let next = step(&all, split, 1).unwrap();
    assert_eq!(next, split + 2);
    assert_eq!(step(&all, next, -1), Some(split), "back to the word's first piece");
    assert_eq!(step(&all, 0, -1), None);
    assert_eq!(step(&all, all.len() - 1, 1), None);
    let down = line_step(&all, 0, 1).unwrap();
    assert!(all[down].rect.y > all[0].rect.y);
    assert_eq!(line_step(&all, down, -1), Some(0), "the nearest word above");
    assert_eq!(find(&all, all[5].start), Some(5));
    assert_eq!(span((all[5].start, all[5].end), (all[2].start, all[2].end)), (all[2].start, all[5].end));
}

#[test]
fn a_point_finds_the_word_on_its_line_or_the_nearest() {
    let b = book(vec![paragraph(TEXT)]);
    let all = words(&b, 300.0);
    let w = all[3].rect;
    assert_eq!(word_at(&all, w.x + w.w / 2.0, w.y + w.h / 2.0), Some(3));
    assert_eq!(word_at(&all, w.x + w.w + 1.0, w.y + 1.0).map(|i| all[i].rect.y), Some(w.y), "between words: one on the line");
    let last = all.last().unwrap().rect;
    assert_eq!(word_at(&all, 0.0, last.y + 500.0).map(|i| all[i].rect.y), Some(last.y), "below the text: the last line");
    assert_eq!(word_at(&[], 0.0, 0.0), None);
}

#[test]
fn table_cells_are_words_of_their_row() {
    let cell = |t: &str| TableCell { text: StyledText { text: t.into(), styles: vec![] }, header: false, col_span: 1 };
    let table = Arc::new(Table { rows: vec![vec![cell("один два"), cell("три")]] });
    let b = book(vec![Block::TableRow { table: table.clone(), row: 0 }]);
    let all = words(&b, 600.0);
    let row_text = "один два\nтри";
    let found: Vec<&str> = all.iter().map(|w| &row_text[w.start.offset..w.end.offset]).collect();
    assert_eq!(found, ["один", "два", "три"]);
    assert_eq!(b.block_range(all[2].start), (Locator::default(), Locator { offset: row_text.len(), ..Locator::default() }));
}
