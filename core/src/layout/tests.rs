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
    let text = "Текст со сноской1 и ссылкой из трёх слов.";
    let note = text.find('1').unwrap();
    let link = text.find("ссылкой").unwrap();
    let styles = vec![
        StyleRange { start: note, end: note + 1, style: Style::NoteRef, target: Some("n1".into()) },
        StyleRange { start: link, end: link + "ссылкой из трёх".len(), style: Style::Link, target: Some("l1".into()) },
    ];
    let b = book(vec![Block::Paragraph { text: StyledText { text: text.into(), styles }, kind: Kind::Body }]);
    let mut t = typesetter(style);
    let ch = paginate(&b, 0, &style, &mut t);
    let page = ch.render(0, &style, &mut t);
    let targets: Vec<&Target> = page.areas.iter().map(|a| &a.target).collect();
    assert_eq!(targets, [&Target::Note("n1".into()), &Target::Link("l1".into())]);
    // The link is one area across its words, drawn in the link ink.
    let area = &page.areas[1];
    assert!(area.rect.w > 120.0 && area.rect.h > 10.0, "{:?}", area.rect);
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

#[test]
fn ideographic_text_breaks_between_characters() {
    let style = PageStyle::for_window(600.0, 900.0, 20.0);
    let text = "智に働けば角が立つ。情に棹させば流される。意地を通せば窮屈だ。とかくに人の世は住みにくい。住みにくさが高じると、安い所へ引き越したくなる。";
    let b = Book { language: Some("ja".into()), ..book(vec![paragraph(text)]) };
    let mut t = Typesetter::new(style, None);
    let layout = t.layout(&b.chapters[0].blocks[0], &b).unwrap();
    let lines = &text_of(&layout).lines;
    assert!(lines.len() >= 2, "the paragraph wraps");
    let right = |line: &Line| line.runs.iter().flat_map(|r| r.positions.iter()).map(|p| p[0]).fold(0.0f32, f32::max);
    for line in lines {
        assert!(right(line) <= style.text_width(), "line ends at {}", right(line));
    }
}

#[test]
fn pages_know_how_far_into_the_book_they_are() {
    let style = PageStyle::for_window(600.0, 500.0, 20.0);
    let mut b = book((0..20).map(|_| paragraph(TEXT)).collect());
    b.chapters.push(b.chapters[0].clone());
    let mut t = typesetter(style);
    let first = paginate(&b, 0, &style, &mut t);
    let second = paginate(&b, 1, &style, &mut t);
    assert_eq!(first.progress(0), 0.0);
    assert!((second.progress(0) - 0.5).abs() < 1e-6, "the second of two equal chapters starts half way");
    let all: Vec<f32> =
        (0..first.pages.len()).map(|i| first.progress(i)).chain((0..second.pages.len()).map(|i| second.progress(i))).collect();
    assert!(all.windows(2).all(|w| w[0] < w[1]), "{all:?}");
    assert!(*all.last().unwrap() < 1.0);
}

#[test]
fn running_head_and_foot() {
    let style = PageStyle::for_window(600.0, 500.0, 20.0);
    let mut b = book((0..20).map(|_| paragraph(TEXT)).collect());
    b.chapters[0].title = Some("Очень длинное название главы, которое никак не поместится в одну строку колонтитула".into());
    let mut t = typesetter(style);
    let ch = paginate(&b, 0, &style, &mut t);
    fn secondary(page: &PageContent) -> Vec<&GlyphRun> {
        page.runs.iter().filter(|r| r.ink == Ink::Secondary).collect()
    }
    assert_eq!(secondary(&ch.render(0, &style, &mut t)).len(), 1, "no running head on the chapter's first page");
    let page = ch.render(1, &style, &mut t);
    let runs = secondary(&page);
    assert_eq!(runs.len(), 2, "running head and foot");
    let head = runs[0];
    let (left, right) = (head.positions[0][0], head.positions.last().unwrap()[0]);
    assert!(left >= style.margin_h - 1.0 && right <= style.width - style.margin_h, "the shortened title fits: {left}…{right}");
    assert!(head.positions[0][1] < style.margin_top, "the head is above the text");
    assert!(ch.render_body(1, &style).runs.iter().all(|r| r.ink != Ink::Secondary), "the body alone has neither");
}

#[test]
fn notes_are_typeset_as_one_tall_page() {
    let mut b = book(vec![paragraph(TEXT)]);
    let note = crate::model::Note { title: Some("1".into()), blocks: (0..3).map(|_| paragraph(TEXT)).collect() };
    b.notes.insert("n1".into(), note);
    let style = PageStyle { height: 10_000.0, margin_top: 16.0, margin_bottom: 16.0, ..PageStyle::for_window(500.0, 10_000.0, 18.0) };
    let mut t = typesetter(style);
    let layout = layout_note(&b, "n1", &style, &mut t).unwrap();
    assert_eq!(layout.pages.len(), 1);
    let page = layout.render_body(0, &style);
    let top = page.runs.iter().flat_map(|r| &r.positions).map(|p| p[1]).fold(f32::MAX, f32::min);
    assert!(top < style.margin_top + 30.0, "the start is not lowered: first baseline at {top}");
    let height = layout.height(0);
    let bottom = page.runs.iter().flat_map(|r| &r.positions).map(|p| p[1]).fold(0.0, f32::max);
    assert!(bottom <= style.margin_top + height && height > 100.0, "height {height}, last baseline {bottom}");
    assert!(layout_note(&b, "missing", &style, &mut t).is_none());
}

/// How many lines the runs make: distinct baselines.
fn line_count(runs: &[GlyphRun]) -> usize {
    let mut ys: Vec<f32> = runs.iter().map(|r| r.positions[0][1]).collect();
    ys.sort_by(f32::total_cmp);
    ys.dedup_by(|a, b| (*a - *b).abs() < 0.01);
    ys.len()
}

#[test]
fn the_column_holds_every_line_once_and_places_round_trip() {
    let style = PageStyle::for_window(600.0, 500.0, 20.0);
    let mut blocks: Vec<Block> = (0..12).map(|_| paragraph(TEXT)).collect();
    blocks.insert(5, Block::EmptyLine);
    let b = book(blocks);
    let mut t = typesetter(style);
    let ch = paginate(&b, 0, &style, &mut t);
    let lines_on_pages: usize = (0..ch.pages.len()).map(|i| line_count(&ch.render_body(i, &style).runs)).sum();
    // The whole column at once: every line exactly once.
    assert_eq!(line_count(&ch.render_flow(0.0, ch.flow_height(), &style).runs), lines_on_pages);
    // Slices of it draw the lines whose tops are inside them, shifted to the slice's top; tiles add up to the column.
    let (y0, y1) = (300.0, 700.0);
    let slice = ch.render_flow(y0, y1, &style);
    for r in &slice.runs {
        let y = r.positions[0][1];
        assert!(y > 0.0 && y < (y1 - y0) + t.line_height, "baseline {y} outside the slice");
    }
    assert!(!slice.runs.is_empty());
    let tile = 333.0;
    let tiles = (0..(ch.flow_height() / tile).ceil() as usize)
        .map(|k| line_count(&ch.render_flow(k as f32 * tile, (k + 1) as f32 * tile, &style).runs));
    assert_eq!(tiles.sum::<usize>(), lines_on_pages, "no line in two tiles, none missing");
    // Page starts are lines of the column and come back.
    for i in 0..ch.pages.len() {
        let at = ch.page_start(i);
        assert_eq!(ch.flow_locator(ch.flow_y(at)), at, "page {i}");
    }
    let ys: Vec<f32> = (0..ch.pages.len()).map(|i| ch.flow_y(ch.page_start(i))).collect();
    assert!(ys.windows(2).all(|w| w[0] < w[1]) && *ys.last().unwrap() < ch.flow_height());
    assert_eq!(ch.flow_locator(1e9), ch.flow_locator(ch.flow_height()), "past the end: the last line");
}
