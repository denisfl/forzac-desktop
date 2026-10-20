use super::*;

#[test]
fn search_by_title_or_author_in_either_alphabet() {
    let (title, authors) = ("Война и мир", "Лев Толстой");
    assert_eq!(rank("война", title, authors, ""), Some(0));
    assert_eq!(rank("мир войн", title, authors, ""), Some(0), "any order, prefixes");
    assert_eq!(rank("voina", title, authors, ""), Some(1), "the title in Latin letters");
    assert_eq!(rank("толс", title, authors, ""), Some(2));
    assert_eq!(rank("tolst", title, authors, ""), Some(3));
    assert_eq!(rank("Dostoevsky", "Бесы", "Фёдор Достоевский", ""), Some(3), "spellings reduced");
    assert_eq!(rank("ФЁДОР", "Бесы", "Фёдор Достоевский", ""), Some(2), "case and ё");
    assert_eq!(rank("chek", "Рассказы", "Антон Чехов", ""), Some(3), "a combination cut short");
    assert_eq!(rank("", title, authors, ""), Some(0));
    assert_eq!(rank("война xyz", title, authors, ""), None);
    assert_eq!(rank("эпоп", title, authors, "Эпопеи"), Some(4), "the series after the authors");
    assert_eq!(rank("epop", title, authors, "Эпопеи"), Some(5));
    assert_eq!(rank("ир", title, authors, ""), None, "the start of a word, not its middle");
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

fn entry(title: &str, authors: &str, series: Option<(&str, Option<f32>)>) -> Entry {
    let series = series.map(|(name, number)| crate::model::Series { name: name.into(), number });
    Entry { title: title.into(), authors: authors.into(), series, ..Entry::default() }
}

fn shown(books: &[Entry], groups: &[Group]) -> Vec<(Option<String>, Vec<String>)> {
    groups.iter().map(|g| (g.heading.clone(), g.books.iter().map(|&i| books[i].title.clone()).collect())).collect()
}

#[test]
fn views_sort_and_group() {
    crate::i18n::set_language(Language::English);
    let books = [
        entry("Стажёры", "Аркадий Стругацкий, Борис Стругацкий", Some(("Полдень, XXII век", Some(4.0)))),
        entry("«Анна Каренина»", "Лев Толстой", None),
        entry("Полдень, XXII век", "Аркадий Стругацкий", Some(("Полдень, XXII век", Some(2.0)))),
        entry("Ёж", "", Some(("Звери", None))),
        entry("Война и мир", "Лев Толстой", None),
        entry("Пикник на обочине", "Аркадий Стругацкий", None),
    ];
    assert_eq!(arrange(&books, View::Recent), [Group { heading: None, books: vec![0, 1, 2, 3, 4, 5] }]);
    let titles: Vec<String> = shown(&books, &arrange(&books, View::Title))[0].1.clone();
    assert_eq!(
        titles,
        ["«Анна Каренина»", "Война и мир", "Ёж", "Пикник на обочине", "Полдень, XXII век", "Стажёры"],
        "ё as е, quotes ignored"
    );
    assert_eq!(
        shown(&books, &arrange(&books, View::Author)),
        [
            (Some("Аркадий Стругацкий".into()), vec!["Полдень, XXII век".into(), "Стажёры".into(), "Пикник на обочине".into()]),
            (Some("Лев Толстой".into()), vec!["«Анна Каренина»".into(), "Война и мир".into()]),
            (Some("Unknown author".into()), vec!["Ёж".into()]),
        ],
        "by surname; a series in order before the rest"
    );
    assert_eq!(
        shown(&books, &arrange(&books, View::Series)),
        [
            (Some("Звери".into()), vec!["Ёж".into()]),
            (Some("Полдень, XXII век".into()), vec!["Полдень, XXII век".into(), "Стажёры".into()]),
            (Some("Not in a series".into()), vec!["«Анна Каренина»".into(), "Война и мир".into(), "Пикник на обочине".into()]),
        ]
    );
    let names = [entry("Б", "Фило В. .", None), entry("А", "Борис Акунин", None), entry("В", "А. Стругацкий", None)];
    let heads: Vec<Option<String>> = arrange(&names, View::Author).into_iter().map(|g| g.heading).collect();
    assert_eq!(heads, [Some("Борис Акунин".into()), Some("А. Стругацкий".into()), Some("Фило В. .".into())], "initials are not surnames");
    assert_eq!(View::from_key(Some("series")), View::Series);
    assert_eq!(View::from_key(None), View::Recent);
    assert_eq!(View::Series.next(false), View::Recent);
    assert_eq!(View::Recent.next(true), View::Series);
}

#[test]
fn the_selection_moves_through_groups() {
    // The card, then a group of 2 in 6 columns, then 8 in 6 columns (rows of 6 and 2).
    let groups = [(1, 1), (2, 6), (8, 6)];
    assert_eq!(step(&groups, 0, 0, 1), 1, "from the card down to the first book");
    assert_eq!(step(&groups, 2, 0, 1), 4, "the second column of the next group");
    assert_eq!(step(&groups, 4, 0, 1), 10, "its second row, same column");
    assert_eq!(step(&groups, 8, 0, 1), 10, "a shorter row: its last book");
    assert_eq!(step(&groups, 10, 0, 1), 10, "the bottom");
    assert_eq!(step(&groups, 10, 0, -1), 4);
    assert_eq!(step(&groups, 9, 0, -1), 3);
    assert_eq!(step(&groups, 4, 0, -1), 2, "up to the previous group's last row, same column");
    assert_eq!(step(&groups, 8, 0, -1), 2, "past its shorter row: the last book");
    assert_eq!(step(&groups, 1, 0, -1), 0, "back to the card");
    assert_eq!(step(&groups, 0, 0, -1), 0);
    assert_eq!(step(&groups, 2, 1, 0), 3, "right across groups in reading order");
    assert_eq!(step(&groups, 0, -1, 0), 0);
    assert_eq!(step(&[], 3, 0, 1), 0);
}

#[test]
fn a_books_information() {
    crate::i18n::set_language(Language::English);
    let book = Book {
        title: "Mort".into(),
        authors: vec!["Terry Pratchett".into()],
        series: Some(crate::model::Series { name: "Discworld".into(), number: Some(4.0) }),
        genres: vec!["Fantasy".into(), "Humour".into()],
        year: Some(1987),
        language: Some("en-GB".into()),
        ..Book::default()
    };
    let info = book_info(&book);
    assert_eq!(info.series, "Discworld · 4");
    assert_eq!(info.details, "Fantasy, Humour · 1987 · English");
    assert_eq!(book_info(&Book { title: "T".into(), ..Book::default() }).details, "");
}
