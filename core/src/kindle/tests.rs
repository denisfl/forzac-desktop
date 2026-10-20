use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::model::{Chapter, Image, Locator, Note, StyleRange, TableCell, TocEntry};

fn text(s: &str) -> StyledText {
    StyledText { text: s.into(), styles: vec![] }
}

fn para(s: &str) -> Block {
    Block::Paragraph { text: text(s), kind: Kind::Body }
}

/// A book with a cover, two chapters (a long one, a note marker, a link, a table) and a note.
fn book() -> Book {
    let mut marked = text("Сказано¹ и сделано.");
    marked.styles.push(StyleRange { start: 14, end: 16, style: Style::NoteRef, target: Some("n1".into()) });
    let mut linked = text("См. вторую главу.");
    linked.styles.push(StyleRange { start: 0, end: linked.text.len(), style: Style::Link, target: Some("to2".into()) });
    let long: String = "Длинный абзац с буквами. ".repeat(400);
    let table = Arc::new(Table {
        rows: vec![vec![
            TableCell { text: text("А"), header: true, col_span: 1 },
            TableCell { text: text("Б"), header: true, col_span: 1 },
        ]],
    });
    let chapters = vec![
        Chapter {
            title: Some("Первая".into()),
            blocks: vec![
                Block::Heading { text: text("Первая"), level: 1 },
                para(&long),
                Block::Paragraph { text: marked, kind: Kind::Body },
                Block::Paragraph { text: linked, kind: Kind::Body },
                Block::TableRow { table, row: 0 },
                Block::Image { id: "pic".into() },
            ],
        },
        Chapter {
            title: Some("Вторая".into()), blocks: vec![Block::Heading { text: text("Вторая <&>"), level: 1 }, para("Конец.")]
        },
    ];
    let jpeg = |n: u8| Image { content_type: "image/jpeg".into(), bytes: vec![0xFF, 0xD8, 0xFF, n] };
    Book {
        id: "0123456789abcdef0123456789abcdef".into(),
        title: "Книга".into(),
        authors: vec!["Анна Автор".into()],
        language: Some("ru".into()),
        annotation: Some("О книге.".into()),
        cover: Some("cover".into()),
        genres: vec!["Fantasy".into()],
        year: Some(2001),
        chapters,
        notes: HashMap::from([("n1".to_owned(), Note { title: Some("1".into()), blocks: vec![para("Текст сноски.")] })]),
        links: HashMap::from([("to2".to_owned(), LinkTarget::Place { start: Locator { chapter: 1, block: 0, offset: 0 }, end: None })]),
        toc: vec![
            TocEntry { title: "Первая".into(), level: 1, target: Locator { chapter: 0, block: 0, offset: 0 } },
            TocEntry { title: "Вторая".into(), level: 1, target: Locator { chapter: 1, block: 0, offset: 0 } },
        ],
        images: HashMap::from([("cover".to_owned(), jpeg(1)), ("pic".to_owned(), jpeg(2))]),
        ..Book::default()
    }
}

/// The records of a PalmDB file.
fn records(file: &[u8]) -> Vec<&[u8]> {
    let be32 = |at: usize| u32::from_be_bytes(file[at..at + 4].try_into().unwrap()) as usize;
    let n = u16::from_be_bytes([file[76], file[77]]) as usize;
    let offsets: Vec<usize> = (0..n).map(|i| be32(78 + 8 * i)).chain([file.len()]).collect();
    offsets.windows(2).map(|w| &file[w[0]..w[1]]).collect()
}

fn be32(r: &[u8], at: usize) -> u32 {
    u32::from_be_bytes(r[at..at + 4].try_into().unwrap())
}

/// The text, the records' trailing entries removed: the TBS (its size in the last byte), then the overlap and its count.
fn text_of(recs: &[&[u8]]) -> Vec<u8> {
    let count = u16::from_be_bytes([recs[0][8], recs[0][9]]) as usize;
    let tbs = be32(recs[0], 16 + 0xe0) & 0b10 != 0;
    let mut out = Vec::new();
    for r in &recs[1..=count] {
        let r = if tbs { &r[..r.len() - (r[r.len() - 1] & 0x7F) as usize] } else { r };
        let overlap = (r[r.len() - 1] & 3) as usize;
        out.extend_from_slice(&r[..r.len() - 1 - overlap]);
    }
    out
}

/// The EXTH records of record 0.
fn exth_of(r0: &[u8]) -> Vec<(u32, Vec<u8>)> {
    let start = 16 + be32(r0, 20) as usize;
    assert_eq!(&r0[start..start + 4], b"EXTH");
    let count = be32(r0, start + 8);
    let mut at = start + 12;
    (0..count)
        .map(|_| {
            let (kind, len) = (be32(r0, at), be32(r0, at + 4) as usize);
            let item = (kind, r0[at + 8..at + len].to_vec());
            at += len;
            item
        })
        .collect()
}

#[test]
fn the_file_is_laid_out_as_calibres_mobi_6() {
    let file = mobi(&book(), &Edits::default());
    assert_eq!(&file[60..68], b"BOOKMOBI");
    let recs = records(&file);
    let r0 = recs[0];
    assert_eq!(&r0[16..20], b"MOBI");
    assert_eq!((be32(r0, 20), be32(r0, 24), be32(r0, 28), be32(r0, 36)), (0xE8, 2, 65001, 6), "header length, book, UTF-8, version 6");
    assert_eq!(be32(r0, 16 + 0x4c), 0x19, "Russian");
    assert_eq!(be32(r0, 16 + 0xe0), 0b11, "multibyte overlap and TBS");
    let text = text_of(&recs);
    assert_eq!(text.len(), be32(r0, 4) as usize, "the text length");
    let count = u16::from_be_bytes([r0[8], r0[9]]) as usize;
    assert!(recs[1..count].iter().all(|r| r.len() > RECORD_SIZE), "full records of 4096 bytes");
    let text = String::from_utf8(text).expect("the records join into UTF-8: a split character is not lost");
    assert!(text.starts_with("<html><head><guide><reference type=\"toc\""));
    assert!(text.contains("Вторая &lt;&amp;&gt;"));
    // Images: the cover first, then the picture; FLIS, FCIS, EOF last.
    let first_image = be32(r0, 16 + 0x5c) as usize;
    assert_eq!(recs[first_image], [0xFF, 0xD8, 0xFF, 1]);
    assert_eq!(recs[first_image + 1], [0xFF, 0xD8, 0xFF, 2]);
    assert_eq!(&recs[recs.len() - 3][..4], b"FLIS");
    assert_eq!(&recs[recs.len() - 2][..4], b"FCIS");
    assert_eq!(recs[recs.len() - 1], b"\xe9\x8e\r\n");
    assert_eq!(be32(r0, 16 + 0xb8) as usize, recs.len() - 2, "the FCIS record");
    let title_at = be32(r0, 16 + 0x44) as usize;
    assert_eq!(&r0[title_at..title_at + "Книга".len()], "Книга".as_bytes());
}

#[test]
fn every_link_leads_to_its_target() {
    let file = mobi(&book(), &Edits::default());
    let text = String::from_utf8(text_of(&records(&file))).unwrap();
    let at = |pos: &str| &text[pos.parse::<usize>().unwrap()..];
    let targets: Vec<&str> = text.split("filepos=").skip(1).map(|s| &s[..10]).collect();
    assert!(targets.len() >= 6, "contents (guide and page), the marker, the note's back link, the link");
    assert!(at(targets[0]).contains("Содержание") && at(targets[0]).starts_with("<p"), "the guide's contents page");
    for t in &targets {
        assert!(at(t).starts_with('<'), "a position at a tag: {}", &at(t)[..20.min(at(t).len())]);
    }
    let marker = text.find("<a filepos=").unwrap();
    let to_note = at(&text[marker + 11..marker + 21]);
    assert!(to_note.starts_with("<p height=\"1em\" width=\"0pt\" align=\"left\"><b><a filepos="), "the marker leads to the note");
    let back = &to_note[to_note.find("filepos=").unwrap() + 8..][..10];
    assert!(
        at(back).starts_with("<a filepos=") && at(back)[22..].starts_with("<sup>"),
        "the note leads back to its marker: {}",
        at(back).chars().take(40).collect::<String>()
    );
    let link = text.find("См. вторую").map(|i| &text[..i]).unwrap();
    let to_second = &link[link.rfind("filepos=").unwrap() + 8..][..10];
    assert!(at(to_second).contains("Вторая &lt;"), "the link leads to the second chapter");
}

#[test]
fn the_details_with_the_readers_edits() {
    crate::i18n::set_language(crate::i18n::Language::English);
    let edits = Edits { title: Some("Своя".into()), authors: Some("Борис Б, Вера В".into()), ..Edits::default() };
    let file = mobi(&book(), &edits);
    let ex = exth_of(records(&file)[0]);
    let all = |kind: u32| ex.iter().filter(|(k, _)| *k == kind).map(|(_, d)| String::from_utf8_lossy(d).into_owned()).collect::<Vec<_>>();
    assert_eq!(all(100), ["Борис Б", "Вера В"]);
    assert_eq!(all(503), ["Своя"]);
    assert_eq!(all(501), ["EBOK"]);
    assert_eq!(all(524), ["ru"]);
    assert_eq!(all(106), ["2001"]);
    assert_eq!(all(105), ["Fantasy"]);
    assert!(ex.iter().any(|(k, d)| *k == 201 && d == &[0, 0, 0, 0]), "the cover is the first image");
}

fn dir() -> PathBuf {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!("forzac-kindle-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn a_book_goes_to_the_connected_kindle() {
    let root = dir();
    std::fs::create_dir_all(root.join("Volumes/USB")).unwrap();
    std::fs::create_dir_all(root.join("Volumes/Kindle/documents")).unwrap();
    std::fs::create_dir_all(root.join("Volumes/Kindle/system")).unwrap();
    assert_eq!(find_in(&[root.join("Volumes")]), Some(root.join("Volumes/Kindle")));
    assert_eq!(find_in(&[root.join("nowhere")]), None);
    let fb2 = r#"<?xml version="1.0" encoding="utf-8"?><FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0"><description><title-info>
        <author><first-name>Анна</first-name><last-name>Автор</last-name></author><book-title>Книга</book-title><lang>ru</lang></title-info></description>
        <body><section><p>Текст.</p></section></body></FictionBook>"#;
    let path = root.join("book.fb2");
    std::fs::write(&path, fb2).unwrap();
    let kindle = root.join("Volumes/Kindle");
    let sent = send(&path, &Edits::default(), &kindle).unwrap();
    assert_eq!(sent, kindle.join("documents/Анна Автор — Книга.mobi"));
    assert_eq!(&std::fs::read(&sent).unwrap()[60..68], b"BOOKMOBI");
    send(&path, &Edits::default(), &kindle).unwrap();
    assert_eq!(std::fs::read_dir(kindle.join("documents")).unwrap().count(), 1, "replaced, no temporary file left");
}

#[test]
fn the_ncx_index_lists_the_contents() {
    let file = mobi(&book(), &Edits::default());
    let recs = records(&file);
    let at = be32(recs[0], 16 + 0xe4) as usize;
    assert_eq!(be32(recs[0], 16 + 0x40) as usize, at, "the first non-book record is the index");
    let (header, record, cncx) = (recs[at], recs[at + 1], recs[at + 2]);
    assert_eq!(&header[..4], b"INDX");
    assert_eq!((be32(header, 24), be32(header, 36), be32(header, 52)), (1, 2, 1), "one record, two entries, one CNCX");
    assert_eq!(&header[192..196], b"TAGX");
    assert_eq!(&header[be32(header, 20) as usize..][..4], b"IDXT");
    assert_eq!(be32(record, 24), 2);
    let text = String::from_utf8(text_of(&recs)).unwrap();
    // The first entry: ident "00", control byte, then the offset as a forward integer.
    let first = &record[192..];
    assert_eq!(&first[..4], &[2, b'0', b'0', 0b1111]);
    let mut offset = 0usize;
    for &b in &first[4..] {
        offset = (offset << 7) | (b & 0x7F) as usize;
        if b & 0x80 != 0 {
            break;
        }
    }
    assert!(text[offset..].contains("Первая") && text[offset..].starts_with("<p"), "the first chapter's heading");
    assert_eq!(&cncx[1..1 + "Первая".len()], "Первая".as_bytes());
}

#[test]
fn trailing_byte_sequences_as_calibre_writes_them() {
    let entries =
        vec![NcxEntry { offset: 1297, length: 28102, title: "a".into() }, NcxEntry { offset: 29399, length: 142344, title: "b".into() }];
    // From the maintainer's Calibre file: a start, a span, an end with a start.
    assert_eq!(tbs(&entries, 0, 4095), [0x82, 0x80]);
    assert_eq!(tbs(&entries, 4096, 8191), [0x83, 0x80, 0x80]);
    assert_eq!(tbs(&entries, 28672, 32767), [0x86, 0x80, 2]);
    assert_eq!(tbs(&entries, 400_000, 404_095), Vec::<u8>::new());
}
