//! The scenarios of `txt-parsing`: the Android app's `TextParserTest` and `PlainTextReaderTest`, ported, so that both
//! apps read a text the same way; and what only the desktop has (file names, the encodings in detail).

use super::*;
use crate::markdown::looks_like;

fn read(text: &str) -> Book {
    crate::parse(text.as_bytes()).unwrap()
}

/// The paragraphs' texts, no-break spaces as spaces (the typography puts them in).
fn paragraphs(book: &Book) -> Vec<String> {
    book.chapters
        .iter()
        .flat_map(|c| &c.blocks)
        .filter_map(|b| if let Block::Paragraph { text, .. } = b { Some(text.text.replace('\u{a0}', " ")) } else { None })
        .collect()
}

// region TextParserTest

#[test]
fn recognizes_text() {
    assert!(is_text("Просто текст".as_bytes()));
    assert!(is_text("# Заголовок".as_bytes()));
    assert!(is_text(&[0xFF, 0xFE, 0x41, 0]));
    assert!(is_text(&encoding_rs::WINDOWS_1251.encode("Отец мой").0));
}

#[test]
fn rejects_non_text() {
    assert!(!is_text(b"%PDF-1.7\n"));
    assert!(!is_text(b"  <!DOCTYPE html><html>"));
    assert!(!is_text(b"<HTML><body>"));
    assert!(!is_text(&[0x42, 0x4F, 0x4F, 0x4B, 0, 0, 0, 1]));
    assert!(!is_text(&[]));
}

#[test]
fn empty_text_has_no_text() {
    assert_eq!(crate::parse(b" \n\n \t\n").err(), Some(Error::NoText));
}

#[test]
fn too_large_is_damaged() {
    assert_eq!(crate::parse(&vec![b'a'; MAX_BYTES + 1]).err(), Some(Error::Damaged));
}

#[test]
fn windows_1251_book() {
    let book =
        crate::parse(&encoding_rs::WINDOWS_1251.encode("   Отец мой Андрей Петрович Гринёв\nв молодости своей служил.\n").0).unwrap();
    assert_eq!(paragraphs(&book)[0], "Отец мой Андрей Петрович Гринёв в молодости своей служил.");
    assert_eq!(book.language.as_deref(), Some("ru"));
}

#[test]
fn strong_signals_make_markdown() {
    assert!(looks_like("Текст\n\n## Раздел\n\nЕщё"));
    assert!(looks_like("Пример:\n\n```\ncode\n```\n"));
    assert!(looks_like("| a | b |\n|---|---|\n| 1 | 2 |"));
}

#[test]
fn two_weak_signals_make_markdown() {
    assert!(looks_like("См. [сайт](https://example.com), это **важно**."));
    assert!(!looks_like("Это **важно**, но и только."));
}

#[test]
fn scene_breaks_are_not_markdown() {
    let novel = "   Первая сцена.\n\n* * *\n\n   Вторая сцена.\n\n* * *\n\n   Третья.\n\n* * *\n\n   Четвёртая.";
    assert!(!looks_like(novel));
    assert_eq!(read(novel).chapters[0].blocks[1], Block::EmptyLine);
}

#[test]
fn hashtag_is_not_heading() {
    assert!(!looks_like("Пост #отпуск и #море"));
}

#[test]
fn inline_backticks_are_not_fence() {
    assert!(!looks_like("Пишите ```так``` нельзя"));
}

// endregion

// region PlainTextReaderTest

#[test]
fn indented_hard_wrapped_paragraphs() {
    let book = read(
        "   Отец мой Андрей Петрович Гринев в молодости своей служил при графе\n\
         Минихе и вышел в отставку премьер-майором в 17.. году.\n\
         \x20  С тех пор жил он в своей Симбирской деревне, где и же-\n\
         нился на девице Авдотье Васильевне Ю.",
    );
    assert_eq!(
        paragraphs(&book),
        [
            "Отец мой Андрей Петрович Гринев в молодости своей служил при графе Минихе и вышел в отставку премьер-майором в 17.. году.",
            "С тех пор жил он в своей Симбирской деревне, где и женился на девице Авдотье Васильевне Ю.",
        ]
    );
}

#[test]
fn blank_line_paragraphs() {
    let book = read("First line of the first\nparagraph continues.\n\nSecond paragraph\nalso wraps.\n");
    assert_eq!(paragraphs(&book), ["First line of the first paragraph continues.", "Second paragraph also wraps."]);
}

#[test]
fn line_per_paragraph() {
    let book = read("Купить хлеба.\nПозвонить маме.\nДописать главу.");
    assert_eq!(paragraphs(&book), ["Купить хлеба.", "Позвонить маме.", "Дописать главу."]);
    assert_eq!(book.chapters.len(), 1);
    assert_eq!(book.chapters[0].title, None);
    assert!(book.toc.is_empty());
    assert_eq!(book.title, "");
}

#[test]
fn blank_separated_single_lines_stay_as_is() {
    let book = read("Первый абзац целиком.\n\nВторой абзац целиком.\n\nТретий.");
    assert_eq!(paragraphs(&book), ["Первый абзац целиком.", "Второй абзац целиком.", "Третий."]);
}

#[test]
fn chapters_from_headings() {
    let book = read("Предисловие автора.\n\nГЛАВА ПЕРВАЯ\n\nТекст первой.\n\nГЛАВА ВТОРАЯ\n\nТекст второй.\n");
    let titles: Vec<Option<&str>> = book.chapters.iter().map(|c| c.title.as_deref()).collect();
    assert_eq!(titles, [None, Some("ГЛАВА ПЕРВАЯ"), Some("ГЛАВА ВТОРАЯ")]);
    let toc: Vec<&str> = book.toc.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(toc, ["ГЛАВА ПЕРВАЯ", "ГЛАВА ВТОРАЯ"]);
    assert_eq!(book.toc[0].target, Locator { chapter: 1, block: 0, offset: 0 });
    assert!(matches!(book.chapters[1].blocks[0], Block::Heading { level: 1, .. }));
}

#[test]
fn numbers_and_english_chapters() {
    let book = read("I\n\nOne.\n\nII\n\nTwo.\n\nChapter 3. The End\n\nThree.");
    let toc: Vec<&str> = book.toc.iter().map(|e| e.title.as_str()).collect();
    assert_eq!(toc, ["I", "II", "Chapter 3. The End"]);
}

#[test]
fn chapter_word_inside_paragraph_does_not_split() {
    let book = read("Он закрыл книгу.\nГлава закончилась внезапно, как и всё.\nДальше было тихо.");
    assert!(book.toc.is_empty());
    assert_eq!(book.chapters.len(), 1);
}

#[test]
fn typography_is_normalized() {
    assert_eq!(paragraphs(&read("- Привет, - сказал он."))[0], "— Привет, — сказал он.");
}

// endregion

#[test]
fn encodings() {
    let text = "Глава первая — «ёжик» и € 𝄞";
    assert_eq!(decode(text.as_bytes()), text, "UTF-8");
    assert_eq!(decode(&[b"\xEF\xBB\xBF", text.as_bytes()].concat()), text, "UTF-8 with its byte order mark");
    let utf16 = |big: bool| {
        let units = text.encode_utf16().flat_map(|u| if big { u.to_be_bytes() } else { u.to_le_bytes() });
        [if big { vec![0xFE, 0xFF] } else { vec![0xFF, 0xFE] }, units.collect()].concat()
    };
    assert_eq!(decode(&utf16(false)), text, "UTF-16 LE");
    assert_eq!(decode(&utf16(true)), text, "UTF-16 BE");
    assert_eq!(decode(b"abc\xE2\x41"), "abcвA", "a broken UTF-8 sequence: windows-1251");
    assert_eq!(decode(b"\xC0\xAF"), "АЇ", "an overlong form is not UTF-8");
    assert_eq!(paragraphs(&read("one\r\ntwo\rthree")), ["one", "two", "three"], "line ends");
}

#[test]
fn binary_files_are_not_books() {
    assert_eq!(crate::parse(b"\xD0\xCF\x11\xE0\xA1\xB1\x1A\xE1\0\0\0\0").err(), Some(Error::NotABook), ".doc");
    assert_eq!(crate::parse(b"BOOKMOBI\0\0\0\x01").err(), Some(Error::NotABook), ".mobi");
    assert_eq!(crate::parse(b"").err(), Some(Error::NotABook));
}

#[test]
fn a_title_from_the_file_name() {
    let dir = std::env::temp_dir().join(format!("forzac-text-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let notes = dir.join("notes");
    std::fs::write(&notes, "Buy milk.\nCall mum.\n").unwrap();
    let book = crate::open(&notes).unwrap();
    assert_eq!(book.title, "notes", "no extension: text all the same");
    assert_eq!(paragraphs(&book), ["Buy milk.", "Call mum."]);
    let shopping = dir.join("Список покупок.txt");
    std::fs::write(&shopping, "Молоко\nХлеб\n").unwrap();
    let book = crate::open(&shopping).unwrap();
    assert_eq!(book.title, "Список покупок");
    assert!(book.authors.is_empty());
    assert_eq!((book.chapters.len(), book.toc.len()), (1, 0));
    std::fs::remove_dir_all(&dir).unwrap();
}
