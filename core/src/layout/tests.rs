use std::collections::HashMap;
use std::sync::Arc;

use super::*;
use crate::model::{Chapter, Image, Kind, StyleRange, Table, TableCell};

const TEXT: &str =
    "Слезы навернулись у нее на глаза. Видно, ей нравилась эта распущенность, видно, ее рассердило, что я хотел пристыдить ее.";

fn paragraph(text: &str) -> Block {
    Block::Paragraph { text: StyledText { text: text.into(), styles: vec![] }, kind: Kind::Body }
}

fn book(blocks: Vec<Block>) -> Book {
    Book { title: "T".into(), language: Some("ru".into()), chapters: vec![Chapter { title: None, blocks }], ..Book::default() }
}

fn typesetter(style: PageStyle) -> Typesetter {
    Typesetter::new(style, Hyphenator::for_language(Some("ru")).map(Arc::new))
}

fn text_of(layout: &BlockLayout) -> &TextLayout {
    match &layout.content {
        Content::Text(t) => t,
        _ => panic!("not text"),
    }
}

/// A PNG header: enough for the typesetter to read the size.
fn png(width: u32, height: u32) -> Image {
    let mut bytes = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    bytes.extend(width.to_be_bytes());
    bytes.extend(height.to_be_bytes());
    bytes.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    Image { content_type: "image/png".into(), bytes }
}

fn cell(text: &str, header: bool) -> TableCell {
    TableCell { text: StyledText { text: text.into(), styles: vec![] }, header, col_span: 1 }
}

#[test]
fn justified_lines_fill_the_column() {
    let style = PageStyle::for_window(600.0, 900.0, 20.0);
    let mut t = typesetter(style);
    let b = book(vec![paragraph(TEXT)]);
    let layout = t.layout(&b.chapters[0].blocks[0], &b).unwrap();
    let text = text_of(&layout);
    assert!(text.lines.len() >= 3);
    // Every line but the last ends at the right edge of the column (a hyphen included).
    let right = |line: &Line| line.runs.iter().flat_map(|r| r.positions.iter()).map(|p| p[0]).fold(0.0f32, f32::max);
    for line in &text.lines[..text.lines.len() - 1] {
        assert!(right(line) > style.text_width() - 30.0 && right(line) <= style.text_width(), "line ends at {}", right(line));
    }
}

#[test]
fn pages_hold_whole_chapter_and_find_locators() {
    let style = PageStyle::for_window(600.0, 500.0, 20.0);
    let b = book((0..30).map(|_| paragraph(TEXT)).collect());
    let mut t = typesetter(style);
    let ch = paginate(&b, 0, &style, &mut t);
    assert!(ch.pages.len() > 3);
    let lines: usize = ch.pages.iter().flat_map(|p| &p.slices).map(|s| s.line_end - s.line_start).sum();
    assert_eq!(lines, ch.blocks.iter().map(|b| b.line_count()).sum::<usize>());
    for i in 0..ch.pages.len() {
        assert_eq!(ch.page_of(ch.page_start(i)), i);
    }
    let page = ch.render(1, &style, &mut t);
    assert!(page.runs.len() > 1);
    assert_eq!(page.runs.last().unwrap().ink, Ink::Secondary, "the page number comes last");
}

#[test]
fn note_markers_and_links_are_active_areas() {
    let style = PageStyle::for_window(600.0, 900.0, 20.0);
    let text = "Текст со сноской1 и ссылкой здесь.";
    let note = text.find('1').unwrap();
    let link = text.find("ссылкой").unwrap();
    let styles = vec![
        StyleRange { start: note, end: note + 1, style: Style::NoteRef, target: Some("n1".into()) },
        StyleRange { start: link, end: link + "ссылкой".len(), style: Style::Link, target: Some("l1".into()) },
    ];
    let b = book(vec![Block::Paragraph { text: StyledText { text: text.into(), styles }, kind: Kind::Body }]);
    let mut t = typesetter(style);
    let ch = paginate(&b, 0, &style, &mut t);
    let page = ch.render(0, &style, &mut t);
    let targets: Vec<&Target> = page.areas.iter().map(|a| &a.target).collect();
    assert_eq!(targets, [&Target::Note("n1".into()), &Target::Link("l1".into())]);
    // The link is one area across the word, drawn in the link ink.
    let area = &page.areas[1];
    assert!(area.rect.w > 40.0 && area.rect.h > 10.0, "{:?}", area.rect);
    assert!(page.runs.iter().any(|r| r.ink == Ink::Link));
    // The marker is raised: its area sits above the link's.
    assert!(page.areas[0].rect.y < area.rect.y);
}

#[test]
fn images_fit_the_column_and_are_never_split() {
    let style = PageStyle::for_window(600.0, 500.0, 20.0);
    let mut blocks: Vec<Block> = (0..3).map(|_| paragraph(TEXT)).collect();
    blocks.push(Block::Image { id: "wide".into() });
    blocks.push(Block::Image { id: "missing".into() });
    blocks.push(paragraph(TEXT));
    let mut b = book(blocks);
    b.images = HashMap::from([("wide".into(), png(2000, 1000))]);
    let mut t = typesetter(style);
    let ch = paginate(&b, 0, &style, &mut t);
    let images: Vec<PageImage> = (0..ch.pages.len()).flat_map(|i| ch.render(i, &style, &mut t).images).collect();
    assert_eq!(images.len(), 1, "a missing image takes no room");
    let r = images[0].rect;
    assert!((r.w - style.text_width()).abs() < 0.01 && (r.h - r.w / 2.0).abs() < 0.01, "{r:?}");
    assert!(r.y + r.h <= style.height - style.margin_bottom + 0.01, "the image is not cut by the page end");
}

#[test]
fn tables_set_rows_with_rules_and_keep_the_header() {
    let style = PageStyle::for_window(600.0, 500.0, 20.0);
    let rows = vec![
        vec![cell("Год", true), cell("Событие", true)],
        vec![cell("1812", false), cell("Отечественная война", false)],
        vec![cell("1861", false), cell("Отмена крепостного права в России", false)],
    ];
    let table = Arc::new(Table { rows });
    let mut blocks: Vec<Block> = vec![paragraph(TEXT)];
    blocks.extend((0..3).map(|row| Block::TableRow { table: table.clone(), row }));
    let b = book(blocks);
    let mut t = typesetter(style);
    let ch = paginate(&b, 0, &style, &mut t);
    let page = ch.render(0, &style, &mut t);
    let strengths: Vec<u8> = page.rules.iter().map(|r| r.strength).collect();
    assert_eq!(strengths, [3, 2, 1, 3], "heavy above and below, medium under the header, light between rows");
    // Rows of one table go without spacing: each row's top rule meets the previous row's bottom.
    assert!(page.rules.iter().all(|r| (r.rect.w - style.text_width()).abs() < 0.01));
    // The header is bold and the second column starts after the first one plus a gap.
    assert!(page.runs.iter().any(|r| r.font == Face::Bold.id()));
    let row = match &ch.blocks[1].content {
        Content::Row { cells, .. } => cells,
        _ => panic!("not a row"),
    };
    assert!(row[1].x > style.text_size, "{}", row[1].x);
}
