//! Ported from the Android app's `SearchTest.kt`.

use super::*;
use crate::model::{Chapter, Kind, Note, StyledText};

fn paragraph(text: &str) -> Block {
    Block::Paragraph { text: StyledText { text: text.into(), styles: Vec::new() }, kind: Kind::Body }
}

fn book(paragraphs: &[&str]) -> Book {
    let heading = Block::Heading { text: StyledText { text: "Глава".into(), styles: Vec::new() }, level: 1 };
    let blocks = std::iter::once(heading).chain(paragraphs.iter().map(|p| paragraph(p))).collect();
    Book { chapters: vec![Chapter { title: Some("Глава".into()), blocks }], ..Book::default() }
}

fn matched(h: &Hit) -> &str {
    &h.snippet[h.matched.clone()]
}

#[test]
fn case_and_yo() {
    let hits = search(&book(&["Ещё немного.", "ЕЩЕ раз и еще."]), "ЕЩЕ");
    assert_eq!(hits.iter().map(matched).collect::<Vec<_>>(), ["Ещё", "ЕЩЕ", "еще"]);
    assert_eq!(
        (hits[0].start, hits[0].end),
        (Locator { chapter: 0, block: 1, offset: 0 }, Locator { chapter: 0, block: 1, offset: "Ещё".len() })
    );
    assert_eq!(hits[0].section.as_deref(), Some("Глава"));
}

#[test]
fn soft_hyphen_and_nbsp() {
    let hits = search(&book(&["Репли\u{AD}кация данных и ещё."]), "репликация");
    assert_eq!(hits.len(), 1, "a soft hyphen does not interrupt a match");
    assert_eq!(matched(&hits[0]), "Репликация");
    assert_eq!(hits[0].end.offset, "Репли\u{AD}кация".len());
    assert_eq!(search(&book(&["и\u{A0}ещё"]), "и ещё").len(), 1, "a non-breaking space equals a space");
    assert_eq!(search(&book(&["и   ещё"]), "и ещё").len(), 1, "a space matches a run of spaces");
}

#[test]
fn snippet_cuts_at_words_with_ellipsis() {
    let long = format!("{}цель {}", "слово ".repeat(30), "хвост ".repeat(30));
    let hits = search(&book(&[long.trim()]), "цель");
    let hit = &hits[0];
    assert!(hit.snippet.starts_with("…слово"), "{}", hit.snippet);
    assert!(hit.snippet.ends_with("хвост…"), "{}", hit.snippet);
    assert_eq!(matched(hit), "цель");
    assert!(hit.snippet.chars().count() < 140);
}

#[test]
fn several_hits_in_a_block_and_short_queries() {
    let b = book(&["кот и кот и кот"]);
    assert_eq!(search(&b, "кот").iter().map(|h| h.start.offset).collect::<Vec<_>>(), [0, "кот и ".len(), "кот и кот и ".len()]);
    assert!(search(&b, "к").is_empty(), "one letter finds nothing");
    assert!(search(&b, "  ").is_empty());
}

#[test]
fn notes_are_not_searched_and_positions_grow() {
    let mut b = book(&["Текст первый.", "Текст второй."]);
    b.notes.insert("n1".into(), Note { title: Some("1".into()), blocks: vec![paragraph("Тайное слово.")] });
    assert!(search(&b, "тайное").is_empty());
    let hits = search(&b, "текст");
    assert!(hits[0].position < hits[1].position);
}

#[test]
fn table_cells() {
    use crate::model::{Table, TableCell};
    use std::sync::Arc;
    let cell = |t: &str| TableCell { text: StyledText { text: t.into(), styles: Vec::new() }, header: false, col_span: 1 };
    let table = Arc::new(Table { rows: vec![vec![cell("Свойство"), cell("Реляционная модель")]] });
    let mut b = book(&["Текст"]);
    b.chapters[0].blocks.push(Block::TableRow { table, row: 0 });
    let hits = search(&b, "реляционная");
    assert_eq!(hits.len(), 1, "found in a cell");
    let offset = "Свойство\n".len();
    assert_eq!((hits[0].start.block, hits[0].start.offset), (2, offset), "an offset into the row's text");
    assert_eq!(matched(&hits[0]), "Реляционная");
    assert!(search(&b, "свойство реляционная").is_empty(), "a match stays in its cell");
}
