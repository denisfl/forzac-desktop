//! Ported from the Android app's `Fb3ParserTest`.

use crate::archive::zip;
use crate::model::{Block, Book, Kind, LinkTarget, Style};

const BOOK: &str = "http://www.fictionbook.org/FictionBook3/relationships/Book";
const BODY: &str = "http://www.fictionbook.org/FictionBook3/relationships/body";
const IMAGE: &str = "http://www.fictionbook.org/FictionBook3/relationships/image";
const CORE: &str = "http://schemas.openxmlformats.org/package/2006/relationships/metadata/core-properties";
const THUMB: &str = "http://schemas.openxmlformats.org/package/2006/relationships/metadata/thumbnail";

fn rels(r: &[(&str, &str, &str)]) -> String {
    let items: String =
        r.iter().map(|(id, kind, target)| format!(r#"<Relationship Id="{id}" Type="{kind}" Target="{target}"/>"#)).collect();
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">{items}</Relationships>"#
    )
}

const DESCRIPTION: &str = r#"<?xml version="1.0" encoding="UTF-8"?><fb3-description xmlns='http://www.fictionbook.org/FictionBook3/description' id='1' version='1.0'>
    <title><main>Анатомия человека</main><sub>учебник</sub></title>
    <fb3-relations><subject id='a' link='author'><title><main>М. Р. Сапин</main></title><first-name>М.</first-name><last-name>Сапин</last-name></subject>
    <subject id='b' link='author'><first-name>Дмитрий</first-name><last-name>Никитюк</last-name></subject>
    <subject id='c' link='translator'><title><main>Переводчик</main></title></subject></fb3-relations>
    <sequence number="7"><title><main>Учебная литература</main></title></sequence>
    <fb3-classification><subject>медицина</subject><subject>анатомия</subject><target-audience age-min="16"/></fb3-classification>
    <lang>ru</lang><written><date value="2011-01-01">2011</date></written>
    <annotation><p>О книге.</p><p>Второй абзац.</p></annotation></fb3-description>"#;

fn body(content: &str) -> String {
    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?><fb3-body xmlns="http://www.fictionbook.org/FictionBook3/body" xmlns:xlink="http://www.w3.org/1999/xlink" id="b">{content}</fb3-body>"#
    )
}

fn pack(body_xml: &str) -> Vec<u8> {
    let root = rels(&[("r1", CORE, "meta/core.xml"), ("r2", BOOK, "fb3/description.xml"), ("r0", THUMB, "/fb3/img/cover.jpg")]);
    let description_rels = rels(&[("rId1", BODY, "body.xml")]);
    let body_rels = rels(&[("img4", IMAGE, "/fb3/img/cells.jpg"), ("lost", IMAGE, "img/none.png")]);
    zip(&[
        ("[Content_Types].xml", br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"/>"#),
        ("_rels/.rels", root.as_bytes()),
        ("meta/core.xml", "<cp:coreProperties xmlns:cp='x' xmlns:dc='y'><dc:title>Другое</dc:title></cp:coreProperties>".as_bytes()),
        ("fb3/description.xml", DESCRIPTION.as_bytes()),
        ("fb3/_rels/description.xml.rels", description_rels.as_bytes()),
        ("fb3/body.xml", body_xml.as_bytes()),
        ("fb3/_rels/body.xml.rels", body_rels.as_bytes()),
        ("fb3/img/cells.jpg", b"JPEG"),
        ("fb3/img/cover.jpg", b"COVER"),
    ])
}

fn paragraphs(book: &Book) -> Vec<(&str, Kind)> {
    book.chapters
        .iter()
        .flat_map(|c| &c.blocks)
        .filter_map(|b| match b {
            Block::Paragraph { text, kind } => Some((text.text.as_str(), *kind)),
            _ => None,
        })
        .collect()
}

#[test]
fn metadata_and_cover() {
    let book = crate::parse(&pack(&body("<section><title><p>Глава</p></title><p>Текст.</p></section>"))).unwrap();
    assert_eq!(book.title, "Анатомия человека");
    assert_eq!(book.authors, ["М. Р. Сапин", "Дмитрий Никитюк"], "title/main, else names; not translators");
    assert_eq!(book.language.as_deref(), Some("ru"));
    assert_eq!(book.annotation.as_deref(), Some("О книге.\nВторой абзац."));
    assert_eq!(book.series, Some(crate::model::Series { name: "Учебная литература".into(), number: Some(7.0) }));
    assert_eq!(book.genres, ["медицина", "анатомия"]);
    assert_eq!(book.year, Some(2011));
    assert_eq!(book.cover.as_deref(), Some("fb3/img/cover.jpg"));
    let cover = &book.images["fb3/img/cover.jpg"];
    assert_eq!((cover.content_type.as_str(), cover.bytes.as_slice()), ("image/jpeg", b"COVER".as_slice()));
}

#[test]
fn body_like_fb2() {
    let book = crate::parse(&pack(&body(
        r##"<title><p>Книга</p></title>
        <section id="s1"><title><p>Первая</p></title>
          <p>
            <strong>Жирный </strong>
            <em>курсив</em> и <underline>подчёркнутый</underline> текст<note href="n_1" xlink:role="footnote">1</note>.
          </p>
          <poem><stanza><p>Строка 1</p><p>Строка 2</p></stanza><stanza><p>Строка 3</p></stanza></poem>
          <ol><li>Раз</li><li><strong>Два</strong></li></ol>
          <ul><li>Пункт</li></ul>
          <blockquote><p>Цитата</p></blockquote>
          <epigraph><p>Эпиграф</p><subscription><p>Автор</p></subscription></epigraph>
          <br/>
          <div float="center"><p><img src="img4"/></p><p>Подпись</p></div>
          <p><img src="lost"/></p>
          <p><a xlink:href="#s1">наверх</a>, <a xlink:href="http://ya.ru/">сайт</a>, <a xlink:href="#nope">битая</a></p>
        </section>
        <notes show="0"><title><p>Примечания</p></title><notebody id="n_1"><title><p>1</p></title><p>Подстраничная сноска</p></notebody></notes>"##,
    )))
    .unwrap();
    let blocks: Vec<&Block> = book.chapters.iter().flat_map(|c| &c.blocks).collect();
    let first = blocks
        .iter()
        .find_map(|b| match b {
            Block::Paragraph { text, .. } if text.text.starts_with("Жирный") => Some(text),
            _ => None,
        })
        .unwrap();
    assert_eq!(first.text, "Жирный курсив и\u{a0}подчёркнутый текст1.", "markup whitespace collapsed");
    let span = |s: &crate::model::StyleRange| &first.text[s.start..s.end];
    assert!(first.styles.iter().any(|s| s.style == Style::Strong && span(s).trim() == "Жирный"));
    assert!(first.styles.iter().any(|s| s.style == Style::Emphasis && span(s) == "курсив"));
    let note = first.styles.iter().find(|s| s.style == Style::NoteRef).unwrap();
    assert_eq!((note.target.as_deref(), span(note)), (Some("n_1"), "1"));
    let n = &book.notes["n_1"];
    assert_eq!(n.title.as_deref(), Some("1"));
    assert!(matches!(&n.blocks[..], [Block::Paragraph { text, .. }] if text.text == "Подстраничная сноска"));

    let ps = paragraphs(&book);
    let verse: Vec<_> = ps.iter().filter(|(t, _)| t.starts_with("Строка")).collect();
    assert!(verse.len() == 3 && verse.iter().all(|(_, k)| *k == Kind::Verse));
    let at = blocks.iter().position(|b| matches!(b, Block::Paragraph { text, .. } if text.text == "Строка 2")).unwrap();
    assert_eq!(blocks[at + 1], &Block::EmptyLine, "an empty line between stanzas");

    assert!(ps.contains(&("1. Раз", Kind::Body)) && ps.contains(&("• Пункт", Kind::Body)));
    let two = blocks
        .iter()
        .find_map(|b| match b {
            Block::Paragraph { text, .. } if text.text == "2. Два" => Some(text),
            _ => None,
        })
        .unwrap();
    assert_eq!(&two.text[two.styles[0].start..two.styles[0].end], "Два", "styles shifted past the marker");
    assert!(ps.contains(&("Цитата", Kind::Cite)) && ps.contains(&("Эпиграф", Kind::Epigraph)) && ps.contains(&("Автор", Kind::TextAuthor)));
    let images: Vec<&Block> = blocks.iter().copied().filter(|b| matches!(b, Block::Image { .. })).collect();
    assert_eq!(images, [&Block::Image { id: "fb3/img/cells.jpg".into() }], "image by relationship; a missing one is dropped");
    assert_eq!(&book.images["fb3/img/cells.jpg"].bytes, b"JPEG");
    assert!(ps.iter().any(|(t, _)| *t == "Подпись"));

    let links = blocks
        .iter()
        .find_map(|b| match b {
            Block::Paragraph { text, .. } if text.text.starts_with("наверх") => Some(text),
            _ => None,
        })
        .unwrap()
        .styles
        .iter()
        .filter(|s| s.style == Style::Link)
        .collect::<Vec<_>>();
    assert_eq!(links.len(), 2, "a dangling link is plain text");
    assert!(matches!(book.links[links[0].target.as_ref().unwrap()], LinkTarget::Place { .. }));
    assert_eq!(book.links[links[1].target.as_ref().unwrap()], LinkTarget::External("http://ya.ru/".into()));
    assert_eq!(book.toc.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(), ["Первая"]);
    assert!(book.chapters.iter().all(|c| c.title.as_deref() != Some("Примечания")), "notes are not a chapter");
}

#[test]
fn package_without_body_is_not_a_book() {
    let root = rels(&[("r2", BOOK, "fb3/description.xml")]);
    let bytes = zip(&[("_rels/.rels", root.as_bytes()), ("fb3/description.xml", DESCRIPTION.as_bytes())]);
    assert_eq!(crate::parse(&bytes).err(), Some(crate::Error::NotABook));
}
