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

#[test]
fn marks_cover_a_match_on_its_line() {
    let text = "Слово перед. Пепперберг дальше идёт текст и ещё текст, чтобы строк было несколько.";
    let b = book(vec![paragraph(text), paragraph(TEXT)]);
    let style = PageStyle::for_window(600.0, 800.0, 20.0);
    let mut t = typesetter(style);
    let layout = paginate(&b, 0, &style, &mut t);
    let hit = &crate::search::search(&b, "пепперберг")[0];
    let marks = layout.marks(0, &style, hit.start, hit.end);
    assert_eq!(marks.len(), 1, "one line");
    let m = marks[0];
    assert!(m.w > 60.0 && m.w < 200.0 && m.x > style.margin_h, "{m:?}");
    // The match's glyphs lie inside the mark: the page's runs have glyphs there on that line.
    let page = layout.render(0, &style, &mut t);
    let inside = page
        .runs
        .iter()
        .flat_map(|r| &r.positions)
        .filter(|p| p[0] >= m.x - 0.5 && p[0] < m.x + m.w && p[1] > m.y && p[1] <= m.y + m.h)
        .count();
    assert!(inside >= "Пепперберг".chars().count(), "{inside}");
    // In the column the same mark, without the page's top margin.
    let flow = layout.flow_marks(0.0, layout.flow_height(), &style, hit.start, hit.end);
    assert_eq!(flow.len(), 1);
    assert!((flow[0].x - m.x).abs() < 0.01);
    assert!(layout.marks(0, &style, crate::model::Locator { chapter: 1, ..hit.start }, hit.end).is_empty(), "another chapter");
}

#[test]
fn code_is_monospace_and_keeps_its_indentation() {
    fonts::set_assets(std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets")));
    let mono = fonts::mono().expect("PT Mono in the assets");
    let style = PageStyle::for_window(600.0, 900.0, 20.0);
    let mut t = typesetter(style);
    let code = Block::Paragraph { text: StyledText { text: "fn main() {\n    run();\n}".into(), styles: vec![] }, kind: Kind::Code };
    let b = book(vec![code]);
    let layout = t.layout(&b.chapters[0].blocks[0], &b).unwrap();
    let text = text_of(&layout);
    assert_eq!(text.lines.len(), 3);
    assert!(text.lines.iter().flat_map(|l| &l.runs).all(|r| r.font == mono), "every glyph from PT Mono");
    // Four spaces of PT Mono at 0.85 × 20 px: 0.6 em each.
    let x = text.lines[1].runs[0].positions[0][0];
    assert!((x - 4.0 * 0.6 * 17.0).abs() < 1.0, "indented by {x}");

    // Inline code: PT Mono inside a serif paragraph.
    let styled =
        StyledText { text: "Call run() now".into(), styles: vec![StyleRange { start: 5, end: 10, style: Style::Code, target: None }] };
    let b = book(vec![Block::Paragraph { text: styled, kind: Kind::Body }]);
    let layout = t.layout(&b.chapters[0].blocks[0], &b).unwrap();
    let runs = &text_of(&layout).lines[0].runs;
    assert!(runs.iter().any(|r| r.font == mono && (r.size - 18.0).abs() < 0.01), "inline code at 0.9");
    assert!(runs.iter().any(|r| r.font != mono));
}

#[test]
fn strikethrough_is_a_stroke_per_line() {
    let style = PageStyle::for_window(600.0, 900.0, 20.0);
    let mut t = typesetter(style);
    let struck = |start: usize, end: usize| StyledText {
        text: TEXT.into(),
        styles: vec![StyleRange { start, end, style: Style::Strikethrough, target: None }],
    };
    // Two words on the first line: one stroke over both, in the text's ink, near the middle of the lower case.
    let two_words = "Слезы навернулись".len();
    let b = book(vec![Block::Paragraph { text: struck(0, two_words), kind: Kind::Body }]);
    let layout = t.layout(&b.chapters[0].blocks[0], &b).unwrap();
    let lines = &text_of(&layout).lines;
    assert_eq!(lines[0].strokes.len(), 1);
    let s = lines[0].strokes[0];
    assert_eq!(s.ink, Ink::Text);
    assert!(s.rect.y < 0.0 && s.rect.y > -10.0 && s.rect.w > 100.0, "{s:?}");
    // The whole paragraph: one stroke on every line.
    let b = book(vec![Block::Paragraph { text: struck(0, TEXT.len()), kind: Kind::Body }]);
    let layout = t.layout(&b.chapters[0].blocks[0], &b).unwrap();
    assert!(text_of(&layout).lines.iter().all(|l| l.strokes.len() == 1));
    // And on the page.
    let page = paginate(&b, 0, &style, &mut t).render_body(0, &style);
    assert_eq!(page.strokes.len(), text_of(&layout).lines.len());
}

#[test]
fn a_match_in_a_table_cell_is_marked_in_the_cell() {
    let style = PageStyle::for_window(600.0, 900.0, 20.0);
    let mut t = typesetter(style);
    let table = Arc::new(Table { rows: vec![vec![cell("Свойство", false), cell("Реляционная", false)]] });
    let b = book(vec![paragraph("Текст"), Block::TableRow { table: table.clone(), row: 0 }]);
    let layout = paginate(&b, 0, &style, &mut t);
    let start = table.cell_starts(0)[1];
    let at = |offset| crate::model::Locator { chapter: 0, block: 1, offset };
    let marks = layout.marks(0, &style, at(start), at(start + "Реляционная".len()));
    assert_eq!(marks.len(), 1);
    // In the second column, right of the first cell.
    assert!(marks[0].x > style.margin_h + 100.0, "{:?}", marks[0]);
}

#[test]
fn the_end_page_shows_the_title_authors_and_finished() {
    let style = PageStyle::for_window(600.0, 900.0, 20.0);
    let mut t = typesetter(style);
    let mut b = book(vec![paragraph(TEXT)]);
    b.title = "Погребение ангела".into();
    b.authors = vec!["Евгений Гришковец".into()];
    let page = end_page(&b, &style, &mut t);
    let ys: Vec<f32> = page.runs.iter().map(|r| r.positions[0][1]).collect();
    assert!(ys.windows(2).all(|w| w[0] < w[1]), "title, authors, then Finished: {ys:?}");
    assert!(ys[0] > 300.0 && *ys.last().unwrap() < 600.0, "in the middle of the window: {ys:?}");
    assert_eq!(page.runs.last().unwrap().ink, Ink::Secondary);
    assert!(page.areas.is_empty() && page.images.is_empty());
}

#[test]
fn wide_images_and_tables_are_areas_for_the_view() {
    let style = PageStyle::for_window(600.0, 900.0, 20.0);
    let link = StyleRange { start: 0, end: 4, style: Style::Link, target: Some("#x".into()) };
    let rows = vec![
        vec![cell("Год", true), cell("Событие", true)],
        vec![
            cell("1812", false),
            TableCell { text: StyledText { text: "Бородино".into(), styles: vec![link] }, header: false, col_span: 1 },
        ],
        vec![cell("1861", false), cell("Реформа", false)],
    ];
    let table = Arc::new(Table { rows });
    let mut blocks = vec![Block::Image { id: "wide".into() }, Block::Image { id: "ornament".into() }, paragraph(TEXT)];
    blocks.extend((0..3).map(|row| Block::TableRow { table: table.clone(), row }));
    let mut b = book(blocks);
    b.images = HashMap::from([("wide".into(), png(400, 200)), ("ornament".into(), png(100, 20))]);
    let mut t = typesetter(style);
    let ch = paginate(&b, 0, &style, &mut t);
    let page = ch.render_body(0, &style);
    let targets: Vec<&Target> = page.areas.iter().map(|a| &a.target).collect();
    let table_at = Locator { chapter: 0, block: 3, offset: 0 };
    assert_eq!(targets, [&Target::Image("wide".into()), &Target::Table(table_at), &Target::Link("#x".into())], "no area for the ornament");
    // One area around the three rows, before the link in a cell.
    let rows: Vec<&Rule> = page.rules.iter().collect();
    let area = page.areas[1].rect;
    assert!((area.y - rows[0].rect.y).abs() < 1.0 && (area.y + area.h - rows.last().unwrap().rect.y).abs() < 1.0, "{area:?}");
    assert!((area.w - style.text_width()).abs() < 0.01);
    // The column has the same areas.
    let flow = ch.render_flow(0.0, ch.flow_height(), &style);
    assert_eq!(flow.areas.iter().map(|a| &a.target).collect::<Vec<_>>(), targets);
    // A note shows the same table without them.
    b.notes.insert("n".into(), crate::model::Note { title: None, blocks: b.chapters[0].blocks.clone() });
    let note = layout_note(&b, "n", &style, &mut t).unwrap();
    assert!(note.render_body(0, &style).areas.iter().all(|a| !a.target.is_viewable()));
}

#[test]
fn a_table_for_the_view_has_columns_by_content() {
    let long = "Отмена крепостного права в России стала одной из самых значительных реформ девятнадцатого века и изменила жизнь миллионов";
    let marker = StyleRange { start: "Москва".len(), end: "Москва1".len(), style: Style::NoteRef, target: Some("n1".into()) };
    let rows = vec![
        vec![cell("Год", true), cell("Событие", true), cell("Место", true)],
        vec![
            cell("1812", false),
            cell(long, false),
            TableCell { text: StyledText { text: "Москва1".into(), styles: vec![marker] }, header: false, col_span: 1 },
        ],
    ];
    let table = Arc::new(Table { rows });
    let mut blocks = vec![paragraph(TEXT)];
    blocks.extend((0..2).map(|row| Block::TableRow { table: table.clone(), row }));
    let b = book(blocks);
    let size = 20.0;
    let style = PageStyle {
        text_size: size,
        margin_h: 0.0,
        margin_top: 0.0,
        margin_bottom: 0.0,
        ..PageStyle::for_window(100_000.0, 100_000.0, size)
    };
    let mut t = typesetter(style);
    assert!(layout_table(&b, Locator { chapter: 0, block: 0, offset: 0 }, &mut t).is_none(), "a paragraph is not a table");
    assert!(layout_table(&b, Locator { chapter: 0, block: 2, offset: 0 }, &mut t).is_none(), "nor a later row");
    let view = layout_table(&b, Locator { chapter: 0, block: 1, offset: 0 }, &mut t).unwrap();
    let g = t.natural_geometry(&table);
    // The long cell's column is capped at 20 em and wraps; the others are as wide as their text.
    assert!((g.w[1] - 20.0 * size).abs() < 0.01, "{:?}", g.w);
    assert!(g.w[0] < 4.0 * size && g.w[2] < 6.0 * size, "{:?}", g.w);
    assert!((view.width - (g.w.iter().sum::<f32>() + 2.0 * size)).abs() < 0.01);
    let rules: Vec<u8> = view.content.rules.iter().map(|r| r.strength).collect();
    assert_eq!(rules, [3, 2, 3]);
    assert!(view.content.rules.iter().all(|r| (r.rect.w - view.width).abs() < 0.01), "rules span the table");
    assert!((view.content.rules.last().unwrap().rect.y + 0.75 - view.height).abs() < 0.01, "the height ends at the last rule");
    let lines = line_count(&view.content.runs);
    assert!(lines >= 4, "the long cell wraps: {lines} lines");
    assert_eq!(view.content.areas.len(), 1);
    assert_eq!(view.content.areas[0].target, Target::Note("n1".into()));
}

#[test]
fn book_pages_are_the_chapters_pages() {
    let style = PageStyle::for_window(600.0, 500.0, 20.0);
    let mut b = book((0..12).map(|_| paragraph(TEXT)).collect());
    b.chapters.push(Chapter { title: Some("Вторая".into()), blocks: (0..30).map(|_| paragraph(TEXT)).collect() });
    b.chapters.push(Chapter { title: None, blocks: vec![paragraph("Конец.")] });
    let mut t = typesetter(style);
    let chapters: Vec<ChapterLayout> = (0..3).map(|c| paginate(&b, c, &style, &mut t)).collect();
    let pages = BookPages::count(&b, style, t.hyphenator(), &CountControl::default()).unwrap();
    assert_eq!(pages.total(), chapters.iter().map(|c| c.pages.len()).sum::<usize>());
    assert_eq!(pages.chapter_starts(), [0, chapters[0].pages.len(), chapters[0].pages.len() + chapters[1].pages.len()]);
    let mut n = 0;
    for ch in &chapters {
        for i in 0..ch.pages.len() {
            assert_eq!(pages.start(n), ch.page_start(i));
            assert_eq!(pages.page_of(ch.page_start(i)), n);
            assert_eq!(pages.of_chapter_page(ch.chapter, i), n);
            n += 1;
        }
    }
    // A place in the middle of a page belongs to it; past the end, the last page.
    let mid = Locator { offset: 5, ..chapters[1].page_start(2) };
    assert_eq!(pages.page_of(mid), chapters[0].pages.len() + 2);
    assert_eq!(pages.page_of(Locator { chapter: 9, ..Locator::default() }), pages.total() - 1);
    assert_eq!(pages.start(10_000), chapters[2].page_start(0));
    // A stopped count gives nothing; a count waits while the reader moves.
    let control = CountControl::default();
    control.stop();
    assert!(BookPages::count(&b, style, None, &control).is_none());
    let control = CountControl::default();
    control.moved();
    let t = std::time::Instant::now();
    assert!(BookPages::count(&b, style, None, &control).is_some());
    assert!(t.elapsed() >= std::time::Duration::from_millis(450), "{:?}", t.elapsed());
}

#[test]
fn heads_of_a_spread() {
    let single = PageStyle::for_window(600.0, 500.0, 20.0);
    let spread = PageStyle { spread: true, ..single };
    let mut b = book((0..20).map(|_| paragraph(TEXT)).collect());
    b.title = "Одноэтажная Америка".into();
    b.chapters[0].title = Some("Глава 1".into());
    let mut t = typesetter(single);
    let ch = paginate(&b, 0, &single, &mut t);
    assert!(ch.pages.len() >= 4);
    let font = t.font_of(Face::Regular);
    let (book_title, chapter) = (t.shaper.shape(font, "Одноэтажная Америка").len(), t.shaper.shape(font, "Глава 1").len());
    let head = |page: &PageContent| page.runs.iter().find(|r| r.ink == Ink::Secondary).unwrap().glyphs.len();
    assert_eq!(head(&ch.render(2, &spread, &mut t)), book_title, "a left page: the book");
    assert_eq!(head(&ch.render(3, &spread, &mut t)), chapter, "a right page: the chapter");
    assert_eq!(head(&ch.render(2, &single, &mut t)), chapter, "a single page: the chapter");
}

#[test]
fn foot_with_book_pages() {
    let style = PageStyle::for_window(600.0, 500.0, 20.0);
    let mut b = book((0..12).map(|_| paragraph(TEXT)).collect());
    b.chapters.push(Chapter { title: None, blocks: (0..12).map(|_| paragraph(TEXT)).collect() });
    let mut t = typesetter(style);
    let pages = BookPages::count(&b, style, t.hyphenator(), &CountControl::default()).unwrap();
    let ch = paginate(&b, 1, &style, &mut t);
    let foot = |page: &PageContent| page.runs.last().unwrap().glyphs.len();
    let font = t.font_of(Face::Regular);
    let expected = format!("{} of {}", pages.chapter_starts()[1] + 2, pages.total());
    assert_eq!(foot(&ch.render_numbered(1, &style, &mut t, Some(&pages))), t.shaper.shape(font, &expected).len());
    // Counted for another window: the chapter's pages as before.
    let other = BookPages::count(&b, PageStyle::for_window(500.0, 500.0, 20.0), None, &CountControl::default()).unwrap();
    let old = format!("2 / {} · {}%", ch.pages.len(), (ch.progress(1) * 100.0).round() as u32);
    assert_eq!(foot(&ch.render_numbered(1, &style, &mut t, Some(&other))), t.shaper.shape(font, &old).len());
}

#[test]
fn list_items_hang_under_their_text() {
    let style = PageStyle::for_window(600.0, 900.0, 20.0);
    let mut t = typesetter(style);
    let item =
        |level: u32| Block::Paragraph { text: StyledText { text: format!("1. {TEXT}"), styles: vec![] }, kind: Kind::ListItem(level) };
    let b = book(vec![item(0), item(1), item(9)]);
    let top = t.layout(&b.chapters[0].blocks[0], &b).unwrap();
    let lines = &text_of(&top).lines;
    assert!(lines.len() >= 2);
    let first = lines[0].runs[0].positions[0][0];
    let second = lines[1].runs[0].positions[0][0];
    assert!(first.abs() < 0.01, "the marker at the edge, no first-line indent: {first}");
    let marker = t.width_of(t.font_of(Face::Regular), "1. ", 20.0);
    assert!((second - marker).abs() < 0.5, "wrapped lines under the text: {second} vs {marker}");
    assert_eq!(top.x, 0.0);
    let nested = t.layout(&b.chapters[0].blocks[1], &b).unwrap();
    assert!((nested.x - 30.0).abs() < 0.01, "a level further right by 1.5 em");
    let deep = t.layout(&b.chapters[0].blocks[2], &b).unwrap();
    assert!((deep.x - 4.0 * 30.0).abs() < 0.01, "at most four levels");
    assert_eq!(top.group, nested.group, "items of one list go without spacing between them");
}
