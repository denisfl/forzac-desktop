use super::*;

#[test]
fn search_by_title_or_author_in_either_alphabet() {
    let (title, authors) = ("Война и мир", "Лев Толстой");
    assert_eq!(rank("война", title, authors), Some(0));
    assert_eq!(rank("мир войн", title, authors), Some(0), "any order, prefixes");
    assert_eq!(rank("voina", title, authors), Some(1), "the title in Latin letters");
    assert_eq!(rank("толс", title, authors), Some(2));
    assert_eq!(rank("tolst", title, authors), Some(3));
    assert_eq!(rank("Dostoevsky", "Бесы", "Фёдор Достоевский"), Some(3), "spellings reduced");
    assert_eq!(rank("ФЁДОР", "Бесы", "Фёдор Достоевский"), Some(2), "case and ё");
    assert_eq!(rank("chek", "Рассказы", "Антон Чехов"), Some(3), "a combination cut short");
    assert_eq!(rank("", title, authors), Some(0));
    assert_eq!(rank("война xyz", title, authors), None);
    assert_eq!(rank("ир", title, authors), None, "the start of a word, not its middle");
}

#[test]
fn continue_reading_is_the_first_unfinished_book() {
    assert_eq!(continue_book([(1.0, true), (0.5, true), (0.2, true)]), Some(1));
    assert_eq!(continue_book([(0.9995, true), (1.0, true)]), None);
    assert_eq!(continue_book([(1.0, true), (0.0, false)]), None, "a book never opened is not continued");
    assert_eq!(continue_book([]), None);
    assert_eq!(share_label(0.0, true), "1%");
    assert_eq!(share_label(0.376, true), "37%");
    assert_eq!(share_label(0.9991, true), tr("Finished"));
    assert_eq!(share_label(0.0, false), tr("New"));
}

#[test]
fn generated_covers_are_stable_and_varied() {
    assert_eq!(java_hash("abc"), 96354);
    assert_eq!(java_hash("Война"), "Война".encode_utf16().fold(0i32, |h, c| h.wrapping_mul(31).wrapping_add(c as i32)));
    let a = cover_style("0123456789abcdef", "Капитанская дочка");
    assert_eq!(a, cover_style("0123456789abcdef", "Капитанская дочка"));
    let styles: Vec<CoverStyle> = (0..1000).map(|i| cover_style(&format!("{i:016x}"), "Title")).collect();
    let marbled = styles.iter().filter(|s| s.marble).count();
    assert!((300..500).contains(&marbled), "about 40 % marbled: {marbled}");
    for cloth in CLOTHS {
        assert!(styles.iter().any(|s| !s.marble && s.cloth == cloth), "every cloth is used");
    }
    let plain = styles.iter().find(|s| !s.marble).unwrap();
    assert!(plain.spine < plain.cloth, "the spine is darker");
    assert_eq!((cover_style("x", "Short").title_scale, cover_style("x", &"a".repeat(60)).title_lines), (1.0, 5));
}

#[test]
fn marbled_paper_is_opaque_and_seeded() {
    let a = marble(42, 20, 30);
    assert_eq!(a.len(), 20 * 30 * 4);
    assert!(a.chunks(4).all(|p| p[3] == 255));
    assert_eq!(a, marble(42, 20, 30));
    assert_ne!(a, marble(43, 20, 30));
    let colours: std::collections::HashSet<&[u8]> = a.chunks(4).collect();
    assert!(colours.len() > 6, "bands with veins");
}

#[test]
fn the_sample_page_in_each_language() {
    assert_eq!(SAMPLE_RU.head, "Капитанская дочка");
    assert_eq!(SAMPLE_EN.paragraphs.len(), 4);
    assert!(SAMPLE_RU.paragraphs.iter().all(|p| !p.contains("  ")), "line continuations leave single spaces");
    assert!(SAMPLE_EN.paragraphs.iter().all(|p| !p.contains("  ")));
}

#[test]
fn soft_hyphens_where_the_patterns_allow() {
    assert_eq!(soft_hyphens("Переносы, да.", "ru"), "Пе\u{ad}ре\u{ad}но\u{ad}сы, да.");
    assert_eq!(soft_hyphens("Переносы", "de"), "Переносы");
}
