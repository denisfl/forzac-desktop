//! One test per requirement of `openspec/specs/fb2-parsing`.

use super::*;

fn fb2(body: &str, rest: &str) -> String {
    format!(
        r##"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0" xmlns:l="http://www.w3.org/1999/xlink">
 <description><title-info><author><first-name>Анна</first-name><last-name>Ахматова</last-name></author>
  <book-title>Книга</book-title><lang>ru</lang>
  <annotation><p>Первая строка.</p><p>Вторая.</p></annotation>
  <coverpage><image l:href="#cover.jpg"/></coverpage></title-info></description>
 <body>{body}</body>
 {rest}
</FictionBook>"##
    )
}

fn book(body: &str, rest: &str) -> Book {
    parse(fb2(body, rest).as_bytes()).unwrap()
}

fn paragraphs(book: &Book, chapter: usize) -> Vec<(String, Kind)> {
    book.chapters[chapter]
        .blocks
        .iter()
        .filter_map(|b| if let Block::Paragraph { text, kind } = b { Some((text.text.clone(), *kind)) } else { None })
        .collect()
}

#[test]
fn metadata() {
    let b = book("<section><p>Текст</p></section>", "");
    assert_eq!(b.title, "Книга");
    assert_eq!(b.authors, ["Анна Ахматова"]);
    assert_eq!(b.language.as_deref(), Some("ru"));
    assert_eq!(b.annotation.as_deref(), Some("Первая строка.\nВторая."));
    assert_eq!(b.cover.as_deref(), Some("cover.jpg"));
}

#[test]
fn zipped_book() {
    let xml = fb2("<section><p>Текст</p></section>", "");
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    zip.start_file("book.fb2", zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated)).unwrap();
    std::io::Write::write_all(&mut zip, xml.as_bytes()).unwrap();
    let bytes = zip.finish().unwrap().into_inner();
    assert_eq!(parse(&bytes).unwrap().chapters.len(), 1);
}

#[test]
fn not_a_book_and_no_text() {
    assert!(matches!(parse(b"<html><body><p>x</p></body></html>"), Err(Error::NotABook)));
    assert!(matches!(parse(b"\x00\x01binary"), Err(Error::NotABook)));
    assert!(matches!(parse(fb2("", "").as_bytes()), Err(Error::NoText)));
}

#[test]
fn encodings() {
    let text = "<section><p>Привет, мир</p></section>";
    for (name, encoding) in [("windows-1251", encoding_rs::WINDOWS_1251), ("KOI8-R", encoding_rs::KOI8_R)] {
        let xml = fb2(text, "").replace("encoding=\"utf-8\"", &format!("encoding=\"{name}\""));
        let (bytes, _, unmappable) = encoding.encode(&xml);
        assert!(!unmappable);
        assert_eq!(paragraphs(&parse(&bytes).unwrap(), 0)[0].0, "Привет, мир", "{name}");
    }
    // UTF-16 LE with a byte order mark.
    let xml = fb2(text, "").replace("encoding=\"utf-8\"", "encoding=\"utf-16\"");
    let mut bytes = vec![0xFF, 0xFE];
    bytes.extend(xml.encode_utf16().flat_map(u16::to_le_bytes));
    assert_eq!(paragraphs(&parse(&bytes).unwrap(), 0)[0].0, "Привет, мир");
}

#[test]
fn entities_without_dtd() {
    let b = book("<section><p>a&nbsp;b &mdash; &laquo;c&raquo; &#169;</p></section>", "");
    assert_eq!(paragraphs(&b, 0)[0].0, "a\u{a0}b\u{a0}— «c» ©");
}

#[test]
fn parts_chapters_and_contents() {
    let b = book(
        r#"<title><p>Книга</p></title>
           <section><title><p>Часть первая</p></title>
             <section><title><p>Глава 1</p></title><p>Раз</p>
               <section><title><p>1.1</p></title><p>Вложенная</p></section></section>
             <section><title><p>Глава 2</p></title><p>Два</p></section>
           </section>"#,
        "",
    );
    let titles: Vec<_> = b.chapters.iter().map(|c| c.title.clone().unwrap_or_default()).collect();
    assert_eq!(titles, ["Книга", "Часть первая. Глава 1", "Глава 2"]);
    let toc: Vec<_> = b.toc.iter().map(|e| (e.title.as_str(), e.level, e.target.chapter, e.target.block)).collect();
    assert_eq!(toc, [("Часть первая", 1, 1, 0), ("Глава 1", 2, 1, 1), ("1.1", 3, 1, 3), ("Глава 2", 2, 2, 0)]);
}

#[test]
fn poem_inside_an_epigraph() {
    let b = book(
        r#"<section><epigraph><poem><stanza><v>Строка</v><v>Ещё</v></stanza><stanza><v>Вторая</v></stanza></poem>
           <text-author>Поэт</text-author></epigraph><p>Текст</p></section>"#,
        "",
    );
    let kinds: Vec<_> = paragraphs(&b, 0).into_iter().collect();
    assert_eq!(
        kinds,
        [
            ("Строка".to_owned(), Kind::Verse),
            ("Ещё".into(), Kind::Verse),
            ("Вторая".into(), Kind::Verse),
            ("Поэт".into(), Kind::TextAuthor),
            ("Текст".into(), Kind::Body)
        ]
    );
    assert!(b.chapters[0].blocks.contains(&Block::EmptyLine), "stanzas are separated");
}

#[test]
fn note_markers_and_notes() {
    let b = book(
        r##"<section><p>Слово<a l:href="#n1" type="note">[1]</a> и <a l:href="#n2" type="note">{2}</a>.</p></section>"##,
        r#"<body name="notes"><title><p>Примечания</p></title>
             <section id="n1"><title><p>1</p></title><p>Первое примечание.</p></section>
             <section><section id="n2"><p>Второе, во вложенной секции.</p></section></section></body>"#,
    );
    let Block::Paragraph { text, .. } = &b.chapters[0].blocks[0] else { panic!() };
    assert_eq!(text.text, "Слово1 и\u{a0}2.");
    let refs: Vec<_> =
        text.styles.iter().filter(|s| s.style == Style::NoteRef).map(|s| (&text.text[s.start..s.end], s.target.as_deref())).collect();
    assert_eq!(refs, [("1", Some("n1")), ("2", Some("n2"))]);
    assert_eq!(b.notes["n1"].title.as_deref(), Some("1"));
    assert_eq!(b.notes["n1"].blocks.len(), 1);
    assert!(b.notes.contains_key("n2"));
}

#[test]
fn links() {
    let b = book(
        r##"<section><p>См. <a l:href="#ch2">главу 2</a>, <a l:href="#q">цитату</a>, <a l:href="https://example.org">сайт</a>,
             <a l:href="#c1">комментарий</a> и <a l:href="#nowhere">пустоту</a>.</p></section>
           <section id="ch2"><title><p>Глава 2</p></title><p>Начало.</p><cite id="q"><p>Цитата</p><p>в две строки</p></cite><p>Конец.</p></section>"##,
        r#"<body name="comments"><section id="c1"><p>Комментарий.</p></section></body>"#,
    );
    let Block::Paragraph { text, .. } = &b.chapters[0].blocks[0] else { panic!() };
    let keys: Vec<_> = text.styles.iter().filter(|s| s.style == Style::Link).map(|s| s.target.clone().unwrap()).collect();
    assert_eq!(keys, ["#ch2", "#q", "https://example.org", "#c1"], "the dangling link is plain text");
    let start = |c, b| Locator { chapter: c, block: b, offset: 0 };
    assert_eq!(b.links["#ch2"], LinkTarget::Place { start: start(1, 0), end: Some(start(1, 5)) });
    assert_eq!(b.links["#q"], LinkTarget::Place { start: start(1, 2), end: Some(start(1, 4)) });
    assert_eq!(b.links["https://example.org"], LinkTarget::External("https://example.org".into()));
    assert_eq!(b.links["#c1"], LinkTarget::Note("c1".into()));
}

#[test]
fn images() {
    let b = book(
        r##"<section><p>Перед</p><image l:href="#pic.png"/><p>Внутри <image l:href="#inline.png"/> абзаца</p><image l:href="#bad"/></section>"##,
        "<binary id=\"pic.png\" content-type=\"image/png\">iVBORw0K\n GgoAAAAN</binary>\
         <binary id=\"inline.png\" content-type=\"image/png\">iVBORw0KGgo</binary>\
         <binary id=\"bad\" content-type=\"image/png\">!!!not base64!!!</binary>",
    );
    let ids: Vec<_> =
        b.chapters[0].blocks.iter().filter_map(|bl| if let Block::Image { id } = bl { Some(id.as_str()) } else { None }).collect();
    assert_eq!(ids, ["pic.png", "inline.png", "bad"]);
    assert_eq!(b.images["pic.png"].content_type, "image/png");
    assert_eq!(&b.images["pic.png"].bytes[..4], b"\x89PNG");
    assert!(b.images.contains_key("inline.png"), "missing padding is accepted");
    assert!(!b.images.contains_key("bad"), "an undecodable image is skipped");
}

#[test]
fn tables() {
    let b = book(
        r#"<section><table><tr><th>Год</th><th>Город</th></tr><tr><td>1912</td><td>Москва</td></tr>
             <tr><td colspan="2">Итого</td></tr></table>
           <table><tr><td><strong>A</strong></td><td><strong>B</strong></td></tr><tr><td>1</td><td>2</td></tr></table>
           <table><tr><td>Врезка</td></tr><tr><td>в одну колонку</td></tr></table></section>"#,
        "",
    );
    let rows: Vec<_> = b.chapters[0]
        .blocks
        .iter()
        .filter_map(|bl| if let Block::TableRow { table, row } = bl { Some((table.clone(), *row)) } else { None })
        .collect();
    assert_eq!(rows.len(), 5);
    let (first, _) = &rows[0];
    assert!(Arc::ptr_eq(first, &rows[2].0), "rows of one table share it");
    assert!(first.rows[0].iter().all(|c| c.header));
    assert_eq!(first.rows[2][0].col_span, 2);
    assert!(rows[3].0.rows[0].iter().all(|c| c.header), "an all-bold first row is the header");
    assert_eq!(paragraphs(&b, 0).iter().map(|(t, _)| t.as_str()).collect::<Vec<_>>(), ["Врезка", "в\u{a0}одну колонку"]);
}

#[test]
fn links_in_table_cells() {
    let b = book(
        r##"<section><table><tr><td>1</td><td><a l:href="#ch2">глава 2</a>, <a l:href="https://example.org">сайт</a></td></tr></table></section>
           <section id="ch2"><title><p>Глава 2</p></title><p>Текст.</p></section>"##,
        "",
    );
    assert!(matches!(b.links.get("#ch2"), Some(LinkTarget::Place { start, .. }) if start.chapter == 1));
    assert_eq!(b.links.get("https://example.org"), Some(&LinkTarget::External("https://example.org".into())));
}

#[test]
fn truncated_book_keeps_what_was_read() {
    let xml = fb2(
        "<section><title><p>1</p></title><p>Один</p></section><section><title><p>2</p></title><p>Два</p></section>\
         <section><title><p>3</p></title><p>Три</p><p>Обрыв посреди",
        "",
    );
    let cut = &xml[..xml.find("Обрыв").unwrap() + "Обрыв".len()];
    let b = parse(cut.as_bytes()).unwrap();
    assert_eq!(b.chapters.len(), 3);
    assert_eq!(paragraphs(&b, 2)[0].0, "Три");
}

#[test]
fn typography() {
    let b = book("<section><p>- Да, - сказал он и ушёл в лес.</p></section>", "");
    assert_eq!(paragraphs(&b, 0)[0].0, "—\u{a0}Да,\u{a0}— сказал он и\u{a0}ушёл в\u{a0}лес.");
}
