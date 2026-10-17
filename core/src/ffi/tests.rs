//! The C ABI as the macOS shell calls it.

use std::ffi::{CStr, CString};

use super::*;

const PARAGRAPH: &str = "Говорили, что на набережной появилось новое лицо: дама с собачкой. Дмитрий Дмитрич Гуров, \
    проживший в Ялте уже две недели и привыкший тут, тоже стал интересоваться новыми лицами.";

fn book_file() -> PathBuf {
    let body: String = (1..=3)
        .map(|i| {
            let text: String = (0..6).map(|_| format!("<p>{PARAGRAPH}</p>")).collect();
            format!(r##"<section id="ch{i}"><title><p>Глава {i}</p></title>{text}</section>"##)
        })
        .collect();
    let fb2 = format!(
        r##"<?xml version="1.0" encoding="utf-8"?>
<FictionBook xmlns="http://www.gribuser.ru/xml/fictionbook/2.0" xmlns:l="http://www.w3.org/1999/xlink">
 <description><title-info><author><first-name>Антон</first-name><last-name>Чехов</last-name></author>
  <book-title>Рассказы</book-title><lang>ru</lang></title-info></description>
 <body><section><p>Сноска<a l:href="#n1" type="note">1</a>, <a l:href="#ch3">глава</a>, <a l:href="https://example.org">сайт</a>.</p></section>{body}</body>
 <body name="notes"><section id="n1"><title><p>1</p></title><p>Верне — кондитерская на набережной.</p></section></body>
</FictionBook>"##
    );
    static N: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let dir =
        std::env::temp_dir().join(format!("forzac-ffi-{}-{}", std::process::id(), N.fetch_add(1, std::sync::atomic::Ordering::SeqCst)));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("book.fb2");
    std::fs::write(&path, fb2).unwrap();
    path
}

fn c(s: &str) -> CString {
    CString::new(s).unwrap()
}

fn string(p: *const c_char) -> Option<String> {
    (!p.is_null()).then(|| unsafe { CStr::from_ptr(p) }.to_string_lossy().into_owned())
}

#[test]
fn a_book_through_the_c_functions() {
    let path = book_file();
    let doc = unsafe { forzac_open(c(path.to_str().unwrap()).as_ptr(), std::ptr::null_mut()) };
    assert!(!doc.is_null());
    unsafe {
        assert_eq!(string(forzac_title(doc)).as_deref(), Some("Рассказы"));
        assert_eq!(string(forzac_authors(doc)).as_deref(), Some("Антон Чехов"));

        let mut n = 0;
        let toc = std::slice::from_raw_parts(forzac_contents(doc, &mut n), n as usize);
        let titles: Vec<String> = toc.iter().map(|e| string(e.title).unwrap()).collect();
        assert_eq!(titles, ["Глава 1", "Глава 2", "Глава 3"]);
        assert!(toc.windows(2).all(|w| w[0].position < w[1].position), "positions grow");

        let settings = forzac_settings_default();
        let pages = forzac_paginate(doc, 1, 600.0, 800.0, &settings);
        assert!(pages >= 1);
        assert!(forzac_page(doc, 0).run_count > 0);
        let start = forzac_page_start(doc, 0);
        assert_eq!(forzac_page_of(doc, start), 0);

        // "Scroll": the chapter as one column, and the chapters around it.
        let height = forzac_flow_height(doc, 2);
        assert!(height > 100.0);
        assert!(forzac_flow(doc, 2, 0.0, height).run_count > 0);
        let middle = forzac_flow_locator(doc, 2, height / 2.0);
        assert_eq!(middle.chapter, 2);
        assert!((forzac_flow_y(doc, middle) - height / 2.0).abs() < 60.0);
        assert_eq!(forzac_page_start(doc, 0).chapter, 1, "the current chapter is still 1");

        // The first chapter holds a note marker and two links.
        forzac_paginate(doc, 0, 600.0, 800.0, &settings);
        let page = forzac_page(doc, 0);
        let areas = std::slice::from_raw_parts(page.areas, page.area_count as usize);
        let targets: Vec<(u8, String)> = areas.iter().map(|a| (a.kind, string(a.target).unwrap())).collect();
        assert!(targets.contains(&(0, "n1".into())), "{targets:?}");
        let (mut at, mut text) = (ForzacLocator { chapter: 0, block: 0, offset: 0 }, std::ptr::null());
        assert_eq!(forzac_link(doc, c("#ch3").as_ptr(), &mut at, &mut text), 1);
        assert_eq!(at.chapter, 3);
        assert_eq!(forzac_link(doc, c("https://example.org").as_ptr(), &mut at, &mut text), 3);
        assert_eq!(string(text).as_deref(), Some("https://example.org"));
        assert_eq!(forzac_link(doc, c("#nowhere").as_ptr(), &mut at, &mut text), 0);
        assert!(forzac_link_preview(doc, c("#ch3").as_ptr(), 500.0, &settings, &mut std::mem::zeroed()) < 0.0, "a chapter is long");

        let mut note = std::mem::zeroed();
        assert!(forzac_note(doc, c("n1").as_ptr(), 500.0, &settings, &mut note) > 20.0);
        assert!(note.run_count > 0);
        assert_eq!(string(forzac_note_title(doc, c("n1").as_ptr())).as_deref(), Some("1"));
        assert!(forzac_note(doc, c("missing").as_ptr(), 500.0, &settings, &mut note) < 0.0);

        assert_eq!(forzac_progress(doc, ForzacLocator { chapter: 0, block: 0, offset: 0 }), 0.0);
        let end = forzac_end_page(doc);
        assert!(end.run_count >= 2 && end.area_count == 0, "title, author, Finished");
        let mut len = 0;
        assert!(forzac_cover(doc, &mut len).is_null());
        forzac_close(doc);
    }
}

#[test]
fn settings_and_recent_books_through_the_state() {
    let path = book_file();
    let state_path = path.with_file_name("state");
    unsafe {
        let state = forzac_state_open(c(state_path.to_str().unwrap()).as_ptr());
        let mut s = forzac_state_settings(state);
        assert_eq!((s.theme, forzac_settings_theme(&s, true), forzac_settings_text_size(&s)), (-1, 2, 20.0));
        forzac_settings_step(&mut s, 1, 1, false); // the next font
        forzac_settings_step(&mut s, 2, 5, false); // sizes stop at the end
        assert_eq!((s.family, forzac_settings_text_size(&s)), (1, 30.0));
        assert!(forzac_state_save_settings(state, &s));
        let doc = forzac_open(c(path.to_str().unwrap()).as_ptr(), std::ptr::null_mut());
        assert!(forzac_state_save_position(state, doc, ForzacLocator { chapter: 2, block: 1, offset: 0 }, false));
        forzac_state_close(state);

        let state = forzac_state_open(c(state_path.to_str().unwrap()).as_ptr());
        let loaded = forzac_state_settings(state);
        assert_eq!((loaded.family, loaded.size, loaded.theme), (s.family, s.size, -1));
        let mut n = 0;
        let recent = std::slice::from_raw_parts(forzac_state_recent(state, 10, &mut n), n as usize);
        assert_eq!(recent.len(), 1);
        assert_eq!(string(recent[0].title).as_deref(), Some("Рассказы"));
        assert!(recent[0].progress > 0.3 && recent[0].progress < 0.7, "chapter 2 of 4: {}", recent[0].progress);
        let mut at = ForzacLocator { chapter: 0, block: 0, offset: 0 };
        assert!(forzac_state_position(state, doc, &mut at));
        assert_eq!((at.chapter, at.block), (2, 1));
        forzac_state_close(state);
        forzac_close(doc);
    }
}

#[test]
fn a_variable_family_reports_its_axes() {
    unsafe {
        forzac_set_assets(c(concat!(env!("CARGO_MANIFEST_DIR"), "/../assets")).as_ptr());
        let doc = forzac_open(c(book_file().to_str().unwrap()).as_ptr(), std::ptr::null_mut());
        let settings = ForzacSettings { family: 1, ..forzac_settings_default() }; // Literata
        forzac_paginate(doc, 1, 600.0, 800.0, &settings);
        let page = forzac_page(doc, 0);
        let font = std::slice::from_raw_parts(page.runs, page.run_count as usize)[0].font; // the title, bold
        assert!(font >= 4, "not the embedded PT Serif");
        let mut buf = [0 as c_char; 4096];
        let mut index = 0;
        assert!(forzac_font_file(font, buf.as_mut_ptr(), buf.len(), &mut index));
        assert!(string(buf.as_ptr()).unwrap().ends_with("Literata.ttf"));
        let (mut tags, mut values) = ([0u32; 8], [0f32; 8]);
        let n = forzac_font_variations(font, tags.as_mut_ptr(), values.as_mut_ptr(), 8) as usize;
        let wght = u32::from_be_bytes(*b"wght");
        assert!(tags[..n].iter().zip(&values[..n]).any(|(t, v)| *t == wght && (*v == 400.0 || *v == 700.0)), "{:?}", &tags[..n]);
        forzac_close(doc);
    }
}

#[test]
fn search_and_marks_through_the_c_functions() {
    unsafe {
        let doc = forzac_open(c(book_file().to_str().unwrap()).as_ptr(), std::ptr::null_mut());
        let book = forzac_book(doc);
        // Searched on another thread, as the shell does (pointers as addresses to cross the thread).
        let address = book as usize;
        let search = std::thread::spawn(move || forzac_search(address as *const BookRef, c("СОБАЧКОЙ").as_ptr()) as usize).join().unwrap()
            as *mut Search;
        forzac_book_release(book);
        let mut n = 0;
        let hits = std::slice::from_raw_parts(forzac_search_hits(search, &mut n), n as usize);
        assert_eq!(hits.len(), 18, "once in each of 6 paragraphs of 3 chapters");
        let h = &hits[0];
        let snippet = string(h.snippet).unwrap();
        assert_eq!(&snippet[h.match_start as usize..h.match_end as usize], "собачкой");
        assert_eq!(string(h.section).as_deref(), Some("Глава 1"));
        let settings = forzac_settings_default();
        forzac_paginate(doc, h.start.chapter, 600.0, 800.0, &settings);
        let page = forzac_page_of(doc, h.start);
        let mut rects = [ForzacRect { x: 0.0, y: 0.0, w: 0.0, h: 0.0 }; 4];
        assert_eq!(forzac_marks(doc, page, h.start, h.end, rects.as_mut_ptr(), 4), 1);
        assert!(rects[0].w > 40.0);
        assert_eq!(forzac_flow_marks(doc, h.start.chapter, 0.0, 100_000.0, h.start, h.end, rects.as_mut_ptr(), 4), 1);
        forzac_search_free(search);
        forzac_close(doc);
    }
}

#[test]
fn errors_and_previews_through_the_c_functions() {
    unsafe {
        let mut error = 0;
        assert!(forzac_open(c("/nonexistent/book.fb2").as_ptr(), &mut error).is_null());
        assert_eq!(error, 5, "unavailable");
        let path = book_file().with_file_name("photo.png");
        std::fs::write(&path, b"\x89PNG not a book").unwrap();
        assert!(forzac_open(c(path.to_str().unwrap()).as_ptr(), &mut error).is_null());
        assert_eq!(error, 1, "not a book");

        // A link to one paragraph: kind 4, typeset for the note sheet.
        let fb2 = r##"<?xml version="1.0" encoding="utf-8"?><FictionBook xmlns:l="http://www.w3.org/1999/xlink">
            <description><title-info><book-title>T</book-title><lang>ru</lang></title-info></description>
            <body><section><p>См. <a l:href="#p2">пояснение</a>.</p><p id="p2">Короткое пояснение.</p></section></body></FictionBook>"##;
        let path = path.with_file_name("links.fb2");
        std::fs::write(&path, fb2).unwrap();
        let doc = forzac_open(c(path.to_str().unwrap()).as_ptr(), &mut error);
        assert!(!doc.is_null());
        let (mut at, mut text) = (ForzacLocator { chapter: 0, block: 0, offset: 0 }, std::ptr::null());
        assert_eq!(forzac_link(doc, c("#p2").as_ptr(), &mut at, &mut text), 4);
        let settings = forzac_settings_default();
        let mut out = std::mem::zeroed();
        assert!(forzac_link_preview(doc, c("#p2").as_ptr(), 500.0, &settings, &mut out) > 10.0);
        assert!(out.run_count > 0);
        forzac_close(doc);
    }
}

#[test]
fn images_and_tables_for_the_view_through_the_c_functions() {
    // A PNG header of 600×300: enough for the typesetter to read the size.
    let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
    png.extend(600u32.to_be_bytes());
    png.extend(300u32.to_be_bytes());
    png.extend([8, 6, 0, 0, 0, 0, 0, 0, 0]);
    let fb2 = format!(
        r##"<?xml version="1.0" encoding="utf-8"?><FictionBook xmlns:l="http://www.w3.org/1999/xlink">
            <description><title-info><book-title>T</book-title><lang>ru</lang></title-info></description>
            <body><section><p>Схема:</p><image l:href="#s.png"/>
            <table><tr><th>Год</th><th>Событие</th></tr><tr><td>1812</td><td>Война<a l:href="#n1" type="note">1</a></td></tr></table>
            </section></body>
            <body name="notes"><section id="n1"><p>Отечественная.</p></section></body>
            <binary id="s.png" content-type="image/png">{}</binary></FictionBook>"##,
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png)
    );
    let path = book_file().with_file_name("view.fb2");
    std::fs::write(&path, fb2).unwrap();
    unsafe {
        let doc = forzac_open(c(path.to_str().unwrap()).as_ptr(), std::ptr::null_mut());
        assert!(!doc.is_null());
        let settings = forzac_settings_default();
        forzac_paginate(doc, 0, 900.0, 1200.0, &settings);
        let page = forzac_page(doc, 0);
        let areas = std::slice::from_raw_parts(page.areas, page.area_count as usize);
        let kinds: Vec<u8> = areas.iter().map(|a| a.kind).collect();
        assert_eq!(kinds, [2, 3, 0], "the image, the table, then the marker in its cell");
        assert_eq!(string(areas[0].target).as_deref(), Some("s.png"));
        let mut len = 0;
        let bytes = forzac_image(doc, areas[0].target, &mut len);
        assert_eq!((bytes, len), (std::slice::from_raw_parts(page.images, 1)[0].bytes, png.len()));

        let table = forzac_table(doc, areas[1].at, 1.0, &settings);
        assert!(!table.is_null());
        let (mut w, mut h) = (0.0, 0.0);
        let view = forzac_table_page(table, &mut w, &mut h);
        assert!(w > 100.0 && h > 30.0 && view.run_count > 0 && view.rule_count > 0, "{w}×{h}");
        let marker = std::slice::from_raw_parts(view.areas, view.area_count as usize);
        assert_eq!((marker.len(), marker[0].kind, string(marker[0].target).as_deref()), (1, 0, Some("n1")));
        // The page view goes on: the table's arrays stay valid.
        forzac_page(doc, 0);
        assert_eq!(forzac_table_page(table, &mut w, &mut h).area_count, 1);
        forzac_table_free(table);
        assert!(forzac_table(doc, ForzacLocator { chapter: 0, block: 0, offset: 0 }, 1.0, &settings).is_null(), "not a table");
        forzac_close(doc);
    }
}

#[test]
fn bookmarks_through_the_c_functions() {
    let path = book_file();
    let state_path = path.with_file_name("bookmarks-state");
    let at = |chapter, block| ForzacLocator { chapter, block, offset: 0 };
    unsafe {
        let state = forzac_state_open(c(state_path.to_str().unwrap()).as_ptr());
        let doc = forzac_open(c(path.to_str().unwrap()).as_ptr(), std::ptr::null_mut());
        let settings = forzac_settings_default();
        let pages = forzac_paginate(doc, 1, 600.0, 800.0, &settings);
        let (start, end) = (forzac_page_start(doc, 0), forzac_page_end(doc, 0));
        assert_eq!(Locator::from(forzac_page_end(doc, pages - 1)), Locator::from(at(2, 0)), "the last page ends at the next chapter");
        assert!(forzac_bookmark_toggle(doc, start, end));
        assert!(forzac_bookmarked(doc, start, end));
        assert!(forzac_bookmark_toggle(doc, at(2, 1), at(2, 2)));
        assert!(forzac_state_save_bookmarks(state, doc));
        forzac_close(doc);

        let doc = forzac_open(c(path.to_str().unwrap()).as_ptr(), std::ptr::null_mut());
        forzac_state_load_bookmarks(state, doc);
        let mut n = 0;
        let places = std::slice::from_raw_parts(forzac_bookmark_places(doc, &mut n), n as usize);
        assert_eq!(places.iter().map(|&p| Locator::from(p)).collect::<Vec<_>>(), [Locator::from(start), Locator::from(at(2, 1))]);
        let list = std::slice::from_raw_parts(forzac_bookmarks(doc, &mut n), n as usize);
        assert_eq!(string(list[1].section).as_deref(), Some("Глава 2"));
        assert!(!string(list[1].snippet).unwrap().is_empty());
        assert!(list[0].position < list[1].position);
        forzac_bookmark_remove(doc, at(2, 1));
        assert!(!forzac_bookmarked(doc, at(2, 0), at(3, 0)));
        assert!(!forzac_bookmark_toggle(doc, start, end), "a marked page is cleared");
        assert!(forzac_state_save_bookmarks(state, doc));
        forzac_close(doc);
        forzac_state_close(state);
    }
}
