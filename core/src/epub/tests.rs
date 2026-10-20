//! Ported from the Android app's `EpubParserTest` (MathML and fixed layout are not goals here).

use crate::archive::zip;
use crate::model::{Block, Book, Kind, LinkTarget, Locator, Style, StyledText};
use crate::{Error, parse};

const CONTAINER_XML: &str = r#"<?xml version="1.0"?>
    <container version="1.0" xmlns="urn:oasis:names:tc:opendocument:xmlns:container">
      <rootfiles><rootfile full-path="OPS/content.opf" media-type="application/oebps-package+xml"/></rootfiles>
    </container>"#;

fn xhtml(body: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
        <!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN" "http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd">
        <html xmlns="http://www.w3.org/1999/xhtml" xmlns:epub="http://www.idpf.org/2007/ops">
        <head><title>t</title><link rel="stylesheet" href="style.css" type="text/css"/></head>
        <body>{body}</body></html>"#
    )
}

/// An EPUB 2 with documents `docs` (name → body) in spine order and an NCX of `ncx` (title → src).
#[derive(Default)]
struct Epub<'a> {
    docs: Vec<(&'a str, String)>,
    ncx: Vec<(&'a str, &'a str)>,
    css: &'a str,
    extra: Vec<(&'a str, String)>,
    manifest: String,
    binaries: Vec<(&'a str, Vec<u8>)>,
}

impl Epub<'_> {
    fn bytes(&self) -> Vec<u8> {
        let id = |name: &str| name.split('.').next().unwrap().replace('/', "_");
        let manifest: String =
            self.docs.iter().map(|(n, _)| format!(r#"<item id="{}" href="{n}" media-type="application/xhtml+xml"/>"#, id(n))).collect();
        let spine: String = self.docs.iter().map(|(n, _)| format!(r#"<itemref idref="{}"/>"#, id(n))).collect();
        let opf = format!(
            r#"<?xml version="1.0" encoding="utf-8"?>
            <package xmlns="http://www.idpf.org/2007/opf" version="2.0" unique-identifier="id">
              <metadata xmlns:dc="http://purl.org/dc/elements/1.1/" xmlns:opf="http://www.idpf.org/2007/opf">
                <dc:title>Книга &amp; тест</dc:title>
                <dc:creator opf:role="aut">Иван  Петров</dc:creator>
                <dc:creator opf:role="trl">Переводчик</dc:creator>
                <dc:creator opf:role="aut">Clear, James</dc:creator>
                <dc:language>ru</dc:language>
                <dc:description>&lt;p&gt;О книге.&lt;/p&gt;</dc:description>
                <meta name="cover" content="cover-img"/>
                <meta name="calibre:series" content="Плоский мир"/><meta name="calibre:series_index" content="3.0"/>
                <dc:subject>Фэнтези</dc:subject><dc:subject>Юмор</dc:subject><dc:subject>Фэнтези</dc:subject>
                <dc:date>1987-11-12T00:00:00+00:00</dc:date>
              </metadata>
              <manifest>{manifest}
                <item id="ncx" href="toc.ncx" media-type="application/x-dtbncx+xml"/>
                <item id="css" href="style.css" media-type="text/css"/>
                <item id="cover-img" href="img/cover.jpg" media-type="image/jpeg"/>
                {}
              </manifest>
              <spine toc="ncx">{spine}</spine>
            </package>"#,
            self.manifest
        );
        let points: String = self
            .ncx
            .iter()
            .map(|(t, src)| format!(r#"<navPoint><navLabel><text>{t}</text></navLabel><content src="{src}"/></navPoint>"#))
            .collect();
        let toc = format!(r#"<?xml version="1.0"?><ncx xmlns="http://www.daisy.org/z3986/2005/ncx/"><navMap>{points}</navMap></ncx>"#);
        let docs: Vec<(String, String)> = self.docs.iter().map(|(n, b)| (format!("OPS/{n}"), xhtml(b))).collect();
        let mut entries: Vec<(&str, &[u8])> = vec![
            ("mimetype", b"application/epub+zip"),
            ("META-INF/container.xml", CONTAINER_XML.as_bytes()),
            ("OPS/content.opf", opf.as_bytes()),
            ("OPS/toc.ncx", toc.as_bytes()),
            ("OPS/style.css", self.css.as_bytes()),
            ("OPS/img/cover.jpg", &[1, 2, 3]),
        ];
        entries.extend(docs.iter().map(|(n, b)| (n.as_str(), b.as_bytes())));
        entries.extend(self.extra.iter().map(|(n, b)| (*n, b.as_bytes())));
        entries.extend(self.binaries.iter().map(|(n, b)| (*n, b.as_slice())));
        zip(&entries)
    }
}

fn docs<'a>(docs: &[(&'a str, &str)]) -> Epub<'a> {
    Epub { docs: docs.iter().map(|(n, b)| (*n, b.to_string())).collect(), ..Epub::default() }
}

fn book(epub: Epub) -> Book {
    parse(&epub.bytes()).unwrap()
}

fn paragraphs(book: &Book) -> Vec<(&StyledText, Kind)> {
    book.chapters
        .iter()
        .flat_map(|c| &c.blocks)
        .filter_map(|b| if let Block::Paragraph { text, kind } = b { Some((text, *kind)) } else { None })
        .collect()
}

fn texts(book: &Book) -> Vec<&str> {
    paragraphs(book).iter().map(|(t, _)| t.text.as_str()).collect()
}

fn span(text: &StyledText, style: Style) -> &str {
    let r = text.styles.iter().find(|s| s.style == style).unwrap();
    &text.text[r.start..r.end]
}

/// The target of the link with text `label`.
fn link<'b>(book: &'b Book, text: &StyledText, label: &str) -> Option<&'b LinkTarget> {
    let r = text.styles.iter().find(|s| s.style == Style::Link && &text.text[s.start..s.end] == label)?;
    book.links.get(r.target.as_ref()?)
}

#[test]
fn metadata() {
    let b = book(docs(&[("a.xhtml", "<p>Текст</p>")]));
    assert_eq!(b.title, "Книга & тест");
    assert_eq!(b.authors, ["Иван Петров", "James Clear"], "a name written for sorting turned back");
    assert_eq!(b.language.as_deref(), Some("ru"));
    assert_eq!(b.annotation.as_deref(), Some("О книге."));
    assert_eq!(b.cover.as_deref(), Some("OPS/img/cover.jpg"));
    assert_eq!(b.images.keys().collect::<Vec<_>>(), ["OPS/img/cover.jpg"]);
    assert_eq!(b.series, Some(crate::model::Series { name: "Плоский мир".into(), number: Some(3.0) }));
    assert_eq!(b.genres, ["Фэнтези", "Юмор"]);
    assert_eq!(b.year, Some(1987));
}

#[test]
fn epub3_collection_is_the_series() {
    let opf = r##"<?xml version="1.0" encoding="utf-8"?>
        <package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">
          <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>T</dc:title><dc:language>en</dc:language>
            <meta property="belongs-to-collection" id="c1">Discworld</meta>
            <meta refines="#c1" property="collection-type">series</meta>
            <meta refines="#c1" property="group-position">2.5</meta></metadata>
          <manifest><item id="p1" href="p1.xhtml" media-type="application/xhtml+xml"/></manifest>
          <spine><itemref idref="p1"/></spine>
        </package>"##;
    let page = xhtml("<p>Text.</p>");
    let b = parse(&zip(&[
        ("META-INF/container.xml", CONTAINER_XML.as_bytes()),
        ("OPS/content.opf", opf.as_bytes()),
        ("OPS/p1.xhtml", page.as_bytes()),
    ]))
    .unwrap();
    let series = b.series.unwrap();
    assert_eq!((series.name.as_str(), series.number_label().as_deref()), ("Discworld", Some("2.5")));
}

#[test]
fn recognised_by_content() {
    assert!(parse(&docs(&[("a.xhtml", "<p>Текст</p>")]).bytes()).is_ok());
    // A zip without `META-INF/container.xml` goes to the FB2 reader: an empty FictionBook is "no text".
    assert_eq!(parse(&zip(&[("book.fb2", b"<FictionBook/>")])).err(), Some(Error::NoText));
    assert_eq!(parse("не zip".as_bytes()).err(), Some(Error::NotABook));
}

#[test]
fn unpacked_size_is_limited() {
    let bytes = docs(&[("a.xhtml", &format!("<p>{}</p>", "Текст. ".repeat(1000)))]).bytes();
    assert!(crate::archive::unzip(&bytes, 1 << 20).is_ok());
    assert_eq!(crate::archive::unzip(&bytes, 4096).err(), Some(Error::NotABook), "a zip bomb is not a book");
}

#[test]
fn converter_headings_and_kinds() {
    let body = r#"
        <div class="title1"><p class="p">Глава 1</p></div>
        <div class="epigraph"><p class="p1">Эпиграф.</p><p class="text-author">Автор</p></div>
        <p class="p1">Текст —  с <em>курсивом</em>.</p>
        <p class="empty-line"/>
        <div class="poem"><div class="stanza"><p class="v">Строка стиха</p></div></div>
        <div class="cite"><p class="p1">Цитата</p></div>
        <p class="subtitle">* * *</p>"#;
    let b = book(docs(&[("a.xhtml", body)]));
    let blocks = &b.chapters[0].blocks;
    assert!(matches!(&blocks[0], Block::Heading { text, level: 1 } if text.text == "Глава 1"));
    let kinds: Vec<Kind> = paragraphs(&b).iter().map(|(_, k)| *k).collect();
    assert_eq!(kinds, [Kind::Epigraph, Kind::TextAuthor, Kind::Body, Kind::Verse, Kind::Cite, Kind::Subtitle]);
    let (text, _) = paragraphs(&b)[2];
    assert_eq!(text.text, "Текст\u{a0}— с\u{a0}курсивом.");
    assert_eq!(span(text, Style::Emphasis), "курсивом");
    assert_eq!(blocks[4], Block::EmptyLine);
}

#[test]
fn heading_levels_normalized() {
    let b = book(docs(&[("a.xhtml", "<h2>Часть</h2><h3>Глава</h3><p>Текст</p>")]));
    let levels: Vec<u32> =
        b.chapters[0].blocks.iter().filter_map(|b| if let Block::Heading { level, .. } = b { Some(*level) } else { None }).collect();
    assert_eq!(levels, [1, 2]);
    assert_eq!(b.chapters[0].title.as_deref(), Some("Часть. Глава"));
}

#[test]
fn italic_from_css() {
    let css = ".calibre15 { font-style: italic } p.strong, .x { font-weight: 700 } div .nested { font-style: italic }";
    let mut e = docs(&[(
        "a.xhtml",
        r#"<p>Слово <span class="calibre15">курсив</span> и <span class="x">жир</span> <span class="nested">нет</span></p>"#,
    )]);
    e.css = css;
    let b = book(e);
    let (text, _) = paragraphs(&b)[0];
    let styles: Vec<(Style, &str)> = text.styles.iter().map(|s| (s.style, &text.text[s.start..s.end])).collect();
    assert_eq!(styles, [(Style::Emphasis, "курсив"), (Style::Strong, "жир")]);
}

#[test]
fn split_documents_are_merged() {
    let mut e = docs(&[
        ("a.xhtml", "<h1>Глава 1</h1><p>Начало</p>"),
        ("a_split.xhtml", "<p>Продолжение</p>"),
        ("b.xhtml", "<p>Без заголовка, но в оглавлении</p>"),
    ]);
    e.ncx = vec![("Глава 1", "a.xhtml"), ("Глава 2", "b.xhtml")];
    let b = book(e);
    assert_eq!(b.chapters.len(), 2);
    let first: Vec<&str> = b.chapters[0].blocks.iter().filter_map(|b| b.text()).map(|t| t.text.as_str()).collect();
    assert_eq!(first, ["Глава 1", "Начало", "Продолжение"]);
    let targets: Vec<Locator> = b.toc.iter().map(|e| e.target).collect();
    assert_eq!(targets, [Locator { chapter: 0, block: 0, offset: 0 }, Locator { chapter: 1, block: 0, offset: 0 }]);
}

#[test]
fn chapters_split_inside_document_at_top_headings() {
    let mut e =
        docs(&[("a.xhtml", r#"<h1 id="p">Предисловие</h1><p>Раз</p><h1 id="c1">Глава первая</h1><h2 id="s">Раздел</h2><p>Два</p>"#)]);
    e.ncx = vec![("Предисловие", "a.xhtml#p"), ("Глава первая", "a.xhtml#c1")];
    let b = book(e);
    assert_eq!(b.chapters.iter().map(|c| c.title.as_deref().unwrap()).collect::<Vec<_>>(), ["Предисловие", "Глава первая. Раздел"]);
    assert_eq!(b.toc[1].target, Locator { chapter: 1, block: 0, offset: 0 });
}

#[test]
fn cover_document_skipped() {
    let b = book(docs(&[("cover.xhtml", r#"<div><img src="img/cover.jpg" alt=""/></div>"#), ("a.xhtml", "<h1>Глава</h1><p>Текст</p>")]));
    assert_eq!(b.chapters.len(), 1);
    assert_eq!(b.chapters[0].title.as_deref(), Some("Глава"));
}

#[test]
fn several_images_are_not_a_cover() {
    let mut e = docs(&[
        ("main.xhtml", r#"<div><img src="img/1.jpg"/><img src="img/2.jpg"/><img src="img/3.jpg"/></div>"#),
        ("readme.xhtml", "<p>О свитке.</p>"),
    ]);
    e.manifest = (1..=3).map(|i| format!(r#"<item id="i{i}" href="img/{i}.jpg" media-type="image/jpeg"/>"#)).collect();
    let b = book(e);
    assert_eq!(b.chapters.iter().flat_map(|c| &c.blocks).filter(|b| matches!(b, Block::Image { .. })).count(), 3);
}

#[test]
fn mislabelled_raster_images_are_recognised_by_content() {
    let mut e = docs(&[(
        "a.xhtml",
        r#"<p>Схема</p><p><img src="img/1.jpg"/><img src="img/2.png"/><img src="img/3.gif"/><img src="img/4.webp"/></p>"#,
    )]);
    e.manifest = ["1.jpg", "2.png", "3.gif", "4.webp", "5.svg", "6.bin"]
        .iter()
        .enumerate()
        .map(|(i, n)| format!(r#"<item id="m{i}" href="img/{n}" media-type="application/octet-stream"/>"#))
        .collect();
    e.binaries = vec![
        ("OPS/img/1.jpg", b"\xFF\xD8\xFF\xE0 jpeg".to_vec()),
        ("OPS/img/2.png", b"\x89PNG\r\n\x1a\n png".to_vec()),
        ("OPS/img/3.gif", b"GIF89a gif".to_vec()),
        ("OPS/img/4.webp", b"RIFF\0\0\0\0WEBPVP8 ".to_vec()),
        ("OPS/img/5.svg", b"<svg xmlns=\"http://www.w3.org/2000/svg\"/>".to_vec()),
        ("OPS/img/6.bin", b"RIFF\0\0\0\0WAVE".to_vec()),
    ];
    let b = book(e);
    let mut types: Vec<(&str, &str)> = b.images.iter().map(|(k, i)| (k.as_str(), i.content_type.as_str())).collect();
    types.sort();
    assert_eq!(
        types,
        [
            ("OPS/img/1.jpg", "image/jpeg"),
            ("OPS/img/2.png", "image/png"),
            ("OPS/img/3.gif", "image/gif"),
            ("OPS/img/4.webp", "image/webp"),
            ("OPS/img/cover.jpg", "image/jpeg"),
        ]
    );
}

#[test]
fn toc_anchors_point_into_chapter() {
    let mut e = docs(&[("a.xhtml", r#"<h1>Глава</h1><p>Раз</p><h2 id="s2">Раздел</h2><p>Два</p>"#)]);
    e.ncx = vec![("Глава", "a.xhtml"), ("Раздел", "a.xhtml#s2")];
    let b = book(e);
    assert_eq!(b.toc[1].target, Locator { chapter: 0, block: 2, offset: 0 });
}

#[test]
fn converter_notes() {
    let b = book(docs(&[
        (
            "a.xhtml",
            r#"<div class="title1"><p>Глава</p></div><p>Челюсти<a href="notes.xhtml#id7" class="a">[1]</a>. Ссылка <a href="b.xhtml#x">куда-то</a>.</p>"#,
        ),
        (
            "notes.xhtml",
            r#"<div class="title"><p>Примечания</p></div><span id="id7"><div class="title1"><p>1</p></div><p class="p1">Идея челюстей.</p></span>"#,
        ),
    ]));
    assert_eq!(b.chapters.len(), 1, "notes are not in the reading flow");
    let (text, _) = paragraphs(&b)[0];
    assert_eq!(text.text, "Челюсти1. Ссылка куда-то.");
    let marker = text.styles.iter().find(|s| s.style == Style::NoteRef).unwrap();
    assert_eq!(&text.text[marker.start..marker.end], "1");
    let note = &b.notes[marker.target.as_ref().unwrap()];
    assert_eq!(note.title.as_deref(), Some("1"));
    assert!(matches!(&note.blocks[..], [Block::Paragraph { text, .. }] if text.text == "Идея челюстей."));
}

#[test]
fn publisher_notes_with_back_links() {
    let b = book(docs(&[
        ("c.xhtml", r#"<h1>Глава</h1><p class="b">Румельт<a href="notes.html#n_10" id="back_n_10"><sup>[10]</sup></a> (14).</p>"#),
        (
            "notes.html",
            r#"<h1 class="master">10 </h1><div class="hsection1" id="n_10"><p class="b">Ричард Румельт.</p><p class="b"><a href="c.xhtml#back_n_10">Вернуться</a></p></div>"#,
        ),
    ]));
    let (text, _) = paragraphs(&b)[0];
    assert_eq!(span(text, Style::NoteRef), "10");
    assert!(text.styles.iter().all(|s| s.style != Style::Superscript), "the superscript inside the marker is gone");
    let marker = text.styles.iter().find(|s| s.style == Style::NoteRef).unwrap();
    let note = &b.notes[marker.target.as_ref().unwrap()];
    assert_eq!(note.blocks.iter().filter_map(|b| b.text()).map(|t| t.text.as_str()).collect::<Vec<_>>(), ["Ричард Румельт."]);
    assert_eq!(b.chapters.len(), 1);
}

#[test]
fn epub3_aside_footnote_hidden_from_flow() {
    let b = book(docs(&[(
        "a.xhtml",
        r##"<h1>Глава</h1><p>Текст<a epub:type="noteref" href="#fn1">*</a> дальше.</p><aside epub:type="footnote" id="fn1"><p>Сноска.</p></aside><p>Конец.</p>"##,
    )]));
    assert_eq!(texts(&b), ["Текст* дальше.", "Конец."]);
    let (text, _) = paragraphs(&b)[0];
    let marker = text.styles.iter().find(|s| s.style == Style::NoteRef).unwrap();
    assert!(matches!(&b.notes[marker.target.as_ref().unwrap()].blocks[..], [Block::Paragraph { text, .. }] if text.text == "Сноска."));
}

#[test]
fn broken_note_ref_becomes_text() {
    let b = book(docs(&[("a.xhtml", r#"<p>Текст<a href="nowhere.xhtml#n1"><sup>1</sup></a></p>"#)]));
    let (text, _) = paragraphs(&b)[0];
    assert_eq!(text.text, "Текст1");
    assert!(text.styles.iter().all(|s| s.style != Style::NoteRef));
}

#[test]
fn lists_pre_and_breaks() {
    let body = "
        <ul><li>Первый</li><li>Второй <b>жирный</b></li></ul>
        <ol start=\"3\"><li>Три</li></ol>
        <pre><code>val x = 1
    if (x) {
}</code></pre>
        <p>Строка<br/>  вторая</p>";
    let b = book(docs(&[("a.xhtml", body)]));
    assert_eq!(texts(&b), ["• Первый", "• Второй жирный", "3. Три", "val x = 1\n    if (x) {\n}", "Строка\nвторая"]);
    assert_eq!(paragraphs(&b)[3].1, Kind::Code);
    assert_eq!(span(paragraphs(&b)[1].0, Style::Strong), "жирный");
}

#[test]
fn list_items_of_paragraphs_keep_their_markers() {
    let b = book(docs(&[(
        "a.xhtml",
        "<ul><li><p>Первый <em>пункт</em></p><p>Продолжение</p></li><li></li><li><p>Второй</p></li></ul><ol><li><p>Раз</p></li></ol>",
    )]));
    assert_eq!(texts(&b), ["• Первый пункт", "Продолжение", "• Второй", "1. Раз"]);
    assert_eq!(span(paragraphs(&b)[0].0, Style::Emphasis), "пункт");
}

#[test]
fn images_resolved_relative_to_document() {
    let b = book(docs(&[(
        "text/a.xhtml",
        r#"<p>До</p><p><img src="../img/pic%201.png"/></p><svg><image xlink:href="../img/cover.jpg"/></svg>"#,
    )]));
    let ids: Vec<&str> =
        b.chapters[0].blocks.iter().filter_map(|b| if let Block::Image { id } = b { Some(id.as_str()) } else { None }).collect();
    assert_eq!(ids, ["OPS/img/pic 1.png", "OPS/img/cover.jpg"]);
}

#[test]
fn nav_toc_with_levels() {
    let nav = xhtml(
        r#"<nav epub:type="toc"><ol><li><a href="a.xhtml">Часть</a><ol><li><a href="a.xhtml#g">Глава</a></li></ol></li></ol></nav><nav epub:type="landmarks"><ol><li><a href="a.xhtml">Начало</a></li></ol></nav>"#,
    );
    let mut e = docs(&[("a.xhtml", r#"<h1>Часть</h1><h2 id="g">Глава</h2><p>Текст</p>"#)]);
    e.extra = vec![("OPS/nav.xhtml", nav)];
    e.manifest = r#"<item id="nav" href="nav.xhtml" media-type="application/xhtml+xml" properties="nav"/>"#.into();
    let b = book(e);
    assert_eq!(b.toc.iter().map(|e| (e.title.as_str(), e.level)).collect::<Vec<_>>(), [("Часть", 1), ("Глава", 2)]);
    assert_eq!(b.toc[1].target, Locator { chapter: 0, block: 1, offset: 0 });
}

#[test]
fn drm_is_reported_and_fonts_are_not_drm() {
    let encryption = |uri: &str| {
        format!(r#"<encryption><EncryptedData><CipherData><CipherReference URI="{uri}"/></CipherData></EncryptedData></encryption>"#)
    };
    let mut e = docs(&[("a.xhtml", "<p>x</p>")]);
    e.extra = vec![("META-INF/encryption.xml", encryption("OPS/a.xhtml"))];
    assert_eq!(parse(&e.bytes()).err(), Some(Error::Protected));
    e.extra = vec![("META-INF/encryption.xml", encryption("OPS/fonts/a.otf"))];
    assert!(parse(&e.bytes()).is_ok());
}

#[test]
fn no_text_is_an_error() {
    assert_eq!(parse(&docs(&[("a.xhtml", r#"<div><img src="img/cover.jpg"/></div>"#)]).bytes()).err(), Some(Error::NoText));
}

#[test]
fn link_to_figure_covers_its_blocks() {
    let b = book(docs(&[(
        "a.xhtml",
        r##"<h1>Глава</h1><p>См. <a href="#fig_2_1">Figure 2-1</a>.</p><figure id="fig_2_1"><img src="img/fig.png"/><figcaption>Рисунок 2-1. Схема.</figcaption></figure><p>Дальше.</p>"##,
    )]));
    let Some(LinkTarget::Place { start, end: Some(end) }) = link(&b, paragraphs(&b)[0].0, "Figure 2-1") else { panic!() };
    let blocks = &b.chapters[start.chapter].blocks[start.block..end.block];
    assert!(
        matches!(blocks, [Block::Image { id }, Block::Paragraph { text, .. }] if id == "OPS/img/fig.png" && text.text == "Рисунок 2-1. Схема.")
    );
}

#[test]
fn link_to_chapter_in_another_document() {
    let b = book(docs(&[
        ("a.xhtml", r#"<h1>Глава 1</h1><p>См. <a href="b.xhtml#ch_consistency">Chapter 10</a>.</p>"#),
        ("b.xhtml", r#"<h1>Глава 10</h1><p>Вступление.</p><h2 id="ch_consistency">Согласованность</h2><p>Текст.</p>"#),
    ]));
    let target = link(&b, paragraphs(&b)[0].0, "Chapter 10").unwrap();
    let at = |chapter, block| Locator { chapter, block, offset: 0 };
    assert_eq!(target, &LinkTarget::Place { start: at(1, 2), end: Some(at(1, 3)) }, "a heading anchor is one block");
}

#[test]
fn link_to_document_start_and_inline_anchor() {
    let b = book(docs(&[
        ("a.xhtml", r#"<h1>Глава 1</h1><p><a href="b.xhtml">Вторая</a> и <a href="b.xhtml#here">здесь</a>.</p>"#),
        ("b.xhtml", r#"<h1>Глава 2</h1><p>Текст <a id="here"/>с якорем.</p>"#),
    ]));
    let p = paragraphs(&b)[0].0;
    let at = |chapter, block| Locator { chapter, block, offset: 0 };
    assert_eq!(link(&b, p, "Вторая"), Some(&LinkTarget::Place { start: at(1, 0), end: None }));
    assert_eq!(link(&b, p, "здесь"), Some(&LinkTarget::Place { start: at(1, 1), end: None }), "an anchor inside a paragraph is a point");
}

#[test]
fn external_links() {
    let b = book(docs(&[(
        "a.xhtml",
        r#"<p>Сервис <a href="https://aws.amazon.com/cloudfront/">CloudFront</a> и <a href="mailto:a@b.c">почта</a>.</p>"#,
    )]));
    let p = paragraphs(&b)[0].0;
    assert_eq!(link(&b, p, "CloudFront"), Some(&LinkTarget::External("https://aws.amazon.com/cloudfront/".into())));
    assert_eq!(link(&b, p, "почта"), Some(&LinkTarget::External("mailto:a@b.c".into())));
}

#[test]
fn links_in_table_cells() {
    let b = book(docs(&[(
        "a.xhtml",
        r#"<table><tr><td>1</td><td><a href="https://example.org">сайт</a></td></tr><tr><td>2</td><td>3</td></tr></table>"#,
    )]));
    assert_eq!(b.links.get("https://example.org"), Some(&LinkTarget::External("https://example.org".into())));
}

#[test]
fn broken_links_become_text() {
    let b = book(docs(&[(
        "a.xhtml",
        r##"<p><a href="#">Chapter 1</a>, <a href="#nowhere">там</a>, <a href="javascript:void(0)">скрипт</a>.</p>"##,
    )]));
    assert!(paragraphs(&b)[0].0.styles.iter().all(|s| s.style != Style::Link));
    assert!(b.links.is_empty());
}

#[test]
fn text_link_to_note_opens_note() {
    let b = book(docs(&[(
        "a.xhtml",
        r##"<h1>Глава</h1><p>Текст<a epub:type="noteref" href="#fn1">*</a>, см. <a href="#fn1">примечание</a>.</p><aside epub:type="footnote" id="fn1"><p>Сноска.</p></aside>"##,
    )]));
    let p = paragraphs(&b)[0].0;
    assert_eq!(p.text, "Текст*, см. примечание.");
    let Some(LinkTarget::Note(id)) = link(&b, p, "примечание") else { panic!() };
    assert!(matches!(&b.notes[id].blocks[..], [Block::Paragraph { text, .. }] if text.text == "Сноска."));
}

#[test]
fn images_directly_in_spine() {
    let opf = r#"<?xml version="1.0" encoding="utf-8"?>
        <package xmlns="http://www.idpf.org/2007/opf" version="3.0" unique-identifier="id">
          <metadata xmlns:dc="http://purl.org/dc/elements/1.1/"><dc:title>T</dc:title><dc:language>en</dc:language></metadata>
          <manifest>
            <item id="p1" href="p01.jpg" media-type="image/jpeg"/>
            <item id="p2" href="p02.xhtml" media-type="application/xhtml+xml"/>
            <item id="p3" href="p03.png" media-type="image/png"/>
            <item id="p4" href="data.bin" media-type="application/octet-stream"/>
          </manifest>
          <spine><itemref idref="p2"/><itemref idref="p1"/><itemref idref="p3"/><itemref idref="p4"/></spine>
        </package>"#;
    let jpeg: Vec<u8> = (0..2000u32).map(|i| (i * 31) as u8).collect();
    let page = xhtml("<p>Текст страницы.</p>");
    let b = parse(&zip(&[
        ("META-INF/container.xml", CONTAINER_XML.as_bytes()),
        ("OPS/content.opf", opf.as_bytes()),
        ("OPS/p02.xhtml", page.as_bytes()),
        ("OPS/data.bin", b"<p>not text</p>"),
        ("OPS/p01.jpg", &jpeg),
        ("OPS/p03.png", &jpeg),
    ]))
    .unwrap();
    let kinds: Vec<&str> =
        b.chapters.iter().flat_map(|c| &c.blocks).map(|b| if let Block::Image { id } = b { id.as_str() } else { "p" }).collect();
    assert_eq!(kinds, ["p", "OPS/p01.jpg", "OPS/p03.png"], "spine order; nothing from image bytes or other types");
}

#[test]
fn ruby_readings_dropped() {
    let b = book(docs(&[("ch1.xhtml", "<p><ruby>三脚几<rp>(</rp><rt>さんきゃくき</rt><rp>)</rp></ruby>に縛りつけた</p>")]));
    assert_eq!(texts(&b), ["三脚几に縛りつけた"]);
}

#[test]
fn tables() {
    let body = r#"
        <p>До таблицы.</p>
        <table id="t1">
          <caption>Таблица 2.1. Сравнение моделей</caption>
          <thead><tr><th>Свойство</th><th>Реляционная</th><td>Документная</td></tr></thead>
          <tbody>
            <tr><td>Схема</td><td><p>При <em>записи</em></p><p>строгая</p></td><td id="cell">При чтении<a href="b.xhtml#n1"><sup>1</sup></a></td></tr>
            <tr><td colspan="3">Итог <table><tr><td>вложенная</td><td>таблица</td></tr></table></td></tr>
            <tr><td></td></tr>
          </tbody>
        </table>
        <table><tr><td> </td></tr></table>
        <p>После.</p>"#;
    let b = book(docs(&[("a.xhtml", body), ("b.xhtml", r#"<aside epub:type="footnote" id="n1"><p>Сноска.</p></aside>"#)]));
    let blocks = &b.chapters[0].blocks;
    let rows: Vec<(usize, &std::sync::Arc<crate::model::Table>, usize)> = blocks
        .iter()
        .enumerate()
        .filter_map(|(i, b)| if let Block::TableRow { table, row } = b { Some((i, table, *row)) } else { None })
        .collect();
    assert_eq!(rows.len(), 3, "an empty row and an empty table are dropped");
    let table = rows[0].1;
    assert!(matches!(&blocks[rows[0].0 - 1], Block::Paragraph { kind: Kind::Subtitle, .. }), "the caption is a subtitle before the table");
    assert!(table.rows[0].iter().all(|c| c.header), "a thead row is a header, td included");
    let cells: Vec<&str> = table.rows[1].iter().map(|c| c.text.text.as_str()).collect();
    assert_eq!(cells, ["Схема", "При записи\nстрогая", "При чтении1"]);
    assert_eq!(span(&table.rows[1][1].text, Style::Emphasis), "записи");
    assert_eq!(span(&table.rows[1][2].text, Style::NoteRef), "1", "a note marker in a cell");
    assert_eq!(table.rows[2][0].col_span, 3);
    assert_eq!(table.rows[2][0].text.text, "Итог\nвложенная\u{a0}— таблица", "a nested table is text of its cell");
    assert_eq!(texts(&b), ["До таблицы.", "Таблица 2.1. Сравнение моделей", "После."]);
}

#[test]
fn single_column_table_is_a_wrapper() {
    let body = r#"<p>Текст.</p><table><tr><td><h3>Врезка</h3><p>Первый абзац.</p><p><img src="img/cover.jpg"/></p><p>Второй.</p></td></tr></table>"#;
    let b = book(docs(&[("a.xhtml", body)]));
    let kinds: Vec<char> = b.chapters[0]
        .blocks
        .iter()
        .map(|b| match b {
            Block::Heading { .. } => 'h',
            Block::Image { .. } => 'i',
            Block::TableRow { .. } => 't',
            _ => 'p',
        })
        .collect();
    assert_eq!(kinds, ['p', 'h', 'p', 'i', 'p'], "content as ordinary blocks, the image included");
}

#[test]
fn truncated_document_keeps_what_was_read() {
    let b = book(docs(&[("a.xhtml", "<h1>Глава</h1><p>Первый абзац.</p><p>Обрыв посреди")]));
    assert_eq!(texts(&b)[0], "Первый абзац.");
}
