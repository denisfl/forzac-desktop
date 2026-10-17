use super::*;
use crate::model::{Block, Chapter, Kind, StyledText, TocEntry};

fn quote(text: &str, section: Option<&str>) -> Entry {
    Entry { start: Locator::default(), end: Locator::default(), text: text.into(), section: section.map(Into::into), position: 0.0 }
}

fn book(title: &str, authors: &[&str]) -> Book {
    Book { title: title.into(), authors: authors.iter().map(|a| a.to_string()).collect(), ..Book::default() }
}

// Android's `QuoteExportTest`.
#[test]
fn quotes_in_text_order_with_chapters() {
    let b = book("Погребение ангела", &["Евгений Гришковец"]);
    let text = export(&b, &[quote("Текст первой цитаты.", Some("Глава 3")), quote("Текст второй цитаты.", Some("Глава 7"))]);
    assert_eq!(text, "Погребение ангела — Евгений Гришковец\n\n> Текст первой цитаты.\n— Глава 3\n\n> Текст второй цитаты.\n— Глава 7");
}

#[test]
fn a_quote_of_two_paragraphs_stays_one_quote() {
    assert_eq!(
        export(&book("Книга", &[]), &[quote("Первый абзац.\nВторой абзац.", Some("Глава 1"))]),
        "Книга\n\n> Первый абзац.\n> Второй абзац.\n— Глава 1"
    );
}

#[test]
fn quotes_without_chapters_and_books_without_titles() {
    assert_eq!(
        export(&book("Книга", &["А", "Б"]), &[quote("Цитата.", None), quote("Ещё.", Some(" "))]),
        "Книга — А, Б\n\n> Цитата.\n\n> Ещё."
    );
    assert_eq!(export(&book("", &[]), &[quote("Цитата.", None)]), "> Цитата.");
    assert_eq!(export(&book(" ", &["Автор"]), &[quote("Цитата.", None)]), "Автор\n\n> Цитата.");
}

#[test]
fn sharing_one_quote_signs_it() {
    assert_eq!(
        share_text(&book("Колыбель для кошки", &["Курт Воннегут", "Другой"]), "Текст."),
        "«Текст.»\n\n— Курт Воннегут, «Колыбель для кошки»"
    );
    assert_eq!(share_text(&book("", &[]), "Текст."), "«Текст.»");
}

#[test]
fn entries_have_the_whole_text_and_the_section() {
    let para = |t: &str| Block::Paragraph { text: StyledText { text: t.into(), styles: vec![] }, kind: Kind::Body };
    let at = |block, offset| Locator { chapter: 0, block, offset };
    let b = Book {
        chapters: vec![Chapter { title: None, blocks: vec![para("Один два три."), para("Четыре\u{AD}пять.")] }],
        toc: vec![TocEntry { title: "Глава 1".into(), level: 1, target: at(0, 0) }],
        ..Book::default()
    };
    let list = entries(&b, &[(at(0, "Один ".len()), at(1, "Четыре\u{AD}пять".len()))]);
    assert_eq!(list[0].text, "два три.\nЧетырепять");
    assert_eq!(list[0].section.as_deref(), Some("Глава 1"));
}
