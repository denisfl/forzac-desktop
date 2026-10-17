use std::sync::Arc;

use super::*;
use crate::hyphen::Hyphenator;
use crate::layout::{PageStyle, Typesetter, paginate};
use crate::model::{Block, Chapter, Kind, Style, StyleRange, StyledText, TocEntry};

const TEXT: &str =
    "Слезы навернулись у нее на глаза. Видно, ей нравилась эта распущенность, видно, ее рассердило, что я хотел пристыдить ее.";

fn at(chapter: usize, block: usize, offset: usize) -> Locator {
    Locator { chapter, block, offset }
}

fn paragraph(text: &str, styles: Vec<StyleRange>) -> Block {
    Block::Paragraph { text: StyledText { text: text.into(), styles }, kind: Kind::Body }
}

#[test]
fn toggling_sets_on_an_unmarked_range_and_clears_a_marked_one() {
    let mut marks = vec![at(0, 5, 0), at(2, 0, 0)];
    assert!(toggle(&mut marks, at(1, 3, 10), at(1, 7, 0)));
    assert_eq!(marks, [at(0, 5, 0), at(1, 3, 10), at(2, 0, 0)], "kept in reading order");
    assert!(marked(&marks, at(1, 0, 0), at(1, 3, 11)));
    assert!(!marked(&marks, at(1, 0, 0), at(1, 3, 10)), "the range's end is not in it");
    // Two bookmarks on one page (set at other text sizes) go together.
    marks.push(at(1, 5, 0));
    marks.sort();
    assert!(!toggle(&mut marks, at(1, 2, 0), at(2, 0, 0)));
    assert_eq!(marks, [at(0, 5, 0), at(2, 0, 0)]);
}

#[test]
fn a_bookmark_marks_the_page_holding_its_text_after_a_re_layout() {
    let book = Book {
        language: Some("ru".into()),
        chapters: vec![Chapter { title: None, blocks: (0..40).map(|_| paragraph(TEXT, vec![])).collect() }],
        ..Book::default()
    };
    // Before any typesetter: the monospace font is looked up once.
    crate::fonts::set_assets(std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets")));
    let layout = |size: f32| {
        let style = PageStyle::for_window(600.0, 500.0, size);
        let mut t = Typesetter::new(style, Hyphenator::for_language(Some("ru")).map(Arc::new));
        paginate(&book, 0, &style, &mut t)
    };
    let small = layout(18.0);
    let mut marks = Vec::new();
    assert!(toggle(&mut marks, small.page_start(5), small.page_end(5)));
    let large = layout(26.0);
    let pages: Vec<usize> = (0..large.pages.len()).filter(|&i| marked(&marks, large.page_start(i), large.page_end(i))).collect();
    assert_eq!(pages, [large.page_of(marks[0])], "exactly the page with the text");
    let last = large.pages.len() - 1;
    assert_eq!(large.page_end(last), at(1, 0, 0), "the last page reaches to the next chapter");
}

#[test]
fn entries_have_the_section_position_and_clean_text() {
    let note = StyleRange { start: 11, end: 12, style: Style::NoteRef, target: Some("n1".into()) };
    let book = Book {
        chapters: vec![
            Chapter { title: None, blocks: vec![paragraph("Intro text", vec![])] },
            Chapter {
                title: None,
                blocks: vec![
                    Block::Heading { text: StyledText { text: "Two".into(), styles: vec![] }, level: 1 },
                    paragraph("Some\u{AD}thing1 here,\u{A0}said", vec![note]),
                    paragraph("Next   line", vec![]),
                    paragraph(&"а".repeat(400), vec![]),
                ],
            },
        ],
        toc: vec![TocEntry { title: "Two".into(), level: 1, target: at(1, 0, 0) }],
        ..Book::default()
    };
    let list = entries(&book, &[at(0, 0, 6), at(1, 0, 0)]);
    assert_eq!(list[0].section, None, "before the first section");
    assert_eq!(list[0].snippet.chars().take(12).collect::<String>(), "text Two Som", "later headings stay");
    assert_eq!(list[1].section.as_deref(), Some("Two"));
    assert!(list[1].snippet.starts_with("Something here, said Next line ааа"), "without the heading: {}", list[1].snippet);
    assert_eq!(list[1].snippet.chars().count(), 160, "160 characters of text, line breaks as spaces");
    assert!(list[0].position < list[1].position);
}

#[test]
fn text_of_a_range_joins_blocks_with_line_breaks() {
    let book = Book {
        chapters: vec![Chapter {
            title: None,
            blocks: vec![paragraph("First one.", vec![]), Block::EmptyLine, paragraph("Second one.", vec![])],
        }],
        ..Book::default()
    };
    assert_eq!(book.text(at(0, 0, 6), at(0, 2, 6), usize::MAX), "one.\nSecond");
    assert_eq!(book.text(at(0, 0, 0), at(0, 0, 5), 3), "Fir");
}
