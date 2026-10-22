//! The stats' Russian texts, in a process of their own: the interface language is global, and the unit tests expect
//! English.

use forzac_core::i18n::{Language, set_language};
use forzac_core::log::{Day, Finish, Log, days_from_civil};
use forzac_core::stats::{Period, book_line, calendar, compute, day_line, figures, month_line};

fn ms(y: i64, m: u32, d: u32) -> u64 {
    days_from_civil(y, m, d) as u64 * 86_400_000 + 12 * 3_600_000
}

#[test]
fn the_stats_speak_russian() {
    set_language(Language::Russian);
    let day = |book: &str, d: &str, minutes: u64| Day {
        book: book.into(),
        time: 1,
        device: "mac".into(),
        day: d.into(),
        seconds: minutes * 60,
        chars: 0,
        pages: 0,
    };
    let finish =
        |book: &str, time| Finish {
            book: book.into(), time, title: "К".into(), authors: String::new(), started: None, approximate: false
        };
    let now = ms(2026, 10, 18);
    let mut log = Log {
        days: vec![day("a", "2026-10-07", 250), day("a", "2026-10-08", 120), day("b", "2026-10-09", 470)],
        finishes: vec![finish("a", now), finish("c", ms(2026, 10, 2))],
        ..Log::default()
    };
    log.first.insert("a".into(), ms(2026, 10, 7));
    assert_eq!(book_line(&log, "a", 7, 0, now, 0), "6 ч · 12 дней · 7 цитат");
    assert_eq!(book_line(&log, "a", 1, 21, now, 0), "6 ч · 12 дней · 1 цитата · 21 закладка");
    assert_eq!(month_line(&log, now, 0), "Октябрь: 2 книги · 14 ч");
    let october = Period::Month { year: 2026, month: 10 };
    assert_eq!(october.title(), "Октябрь 2026");
    let f = figures(&compute(october, &log, 0));
    assert_eq!(f.map(|(_, caption)| caption), ["за чтением", "книги прочитано", "дня с\u{a0}книгой"]);
    assert_eq!(day_line(days_from_civil(2026, 10, 12), 48), "12 октября · 48 мин");
    assert_eq!(day_line(days_from_civil(2026, 10, 13), 0), "13 октября · не читали");
    let month = calendar(october, 300.0, 1.0, 3);
    assert_eq!(month.labels.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["пн", "вт", "ср", "чт", "пт", "сб", "вс"]);
    assert_eq!(calendar(Period::Year(2026), 900.0, 1.0, 3).labels[4].text, "Май");
}
