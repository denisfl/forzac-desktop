use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::model::{Block, Chapter, Kind, StyledText};

const MIN: u64 = 60_000;
/// 2026-10-18 00:00 UTC.
const DAY0: u64 = 1_792_281_600_000;
/// Moscow: three hours east of UTC.
const MSK: i64 = 3 * 3600;

fn store() -> Store {
    static N: AtomicUsize = AtomicUsize::new(0);
    let dir = std::env::temp_dir().join(format!("forzac-log-{}-{}", std::process::id(), N.fetch_add(1, Ordering::SeqCst)));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    Store::open(dir.join("state"))
}

fn book() -> Book {
    let p = |t: &str| Block::Paragraph { text: StyledText { text: t.into(), styles: Vec::new() }, kind: Kind::Body };
    let chapter = |n: usize| Chapter { title: None, blocks: (0..10).map(|i| p(&format!("{n}{i}ёлки{}", "х".repeat(994)))).collect() };
    Book {
        id: "b".into(),
        title: "Книга".into(),
        authors: vec!["Автор".into()],
        chapters: (0..12).map(chapter).collect(),
        ..Book::default()
    }
}

fn at(chapter: usize, block: usize) -> Locator {
    Locator { chapter, block, offset: 0 }
}

fn seconds(sums: &[Sum]) -> Vec<(String, u64)> {
    sums.iter().map(|s| (s.day.clone(), s.seconds)).collect()
}

#[test]
fn a_slow_page_counts_in_full() {
    let mut t = Tally::default();
    t.active(DAY0 + 10 * 3_600_000, 0);
    t.active(DAY0 + 10 * 3_600_000 + 3 * MIN, 0);
    assert_eq!(seconds(&t.drain()), [("2026-10-18".to_owned(), 180)]);
}

#[test]
fn a_gap_counts_at_most_five_minutes() {
    let mut t = Tally::default();
    t.active(DAY0, 0);
    t.active(DAY0 + 40 * MIN, 0);
    assert_eq!(t.drain()[0].seconds, 300);
}

#[test]
fn reading_past_midnight_is_split_by_the_local_day() {
    let mut t = Tally::default();
    // 23:50 to 00:10 in Moscow, a key every 5 minutes.
    let start = DAY0 + 20 * 3_600_000 + 50 * MIN;
    for i in 0..=4 {
        t.active(start + i * 5 * MIN, MSK);
    }
    assert_eq!(seconds(&t.drain()), [("2026-10-18".to_owned(), 600), ("2026-10-19".to_owned(), 600)]);
}

#[test]
fn after_a_pause_the_next_gap_does_not_count() {
    let mut t = Tally::default();
    t.active(DAY0, 0);
    t.paused(DAY0 + MIN, 0);
    t.active(DAY0 + 61 * MIN, 0);
    t.active(DAY0 + 62 * MIN, 0);
    assert_eq!(t.drain()[0].seconds, 120);
}

#[test]
fn a_fraction_of_a_second_waits_for_the_next_drain() {
    let mut t = Tally::default();
    t.active(DAY0, 0);
    t.active(DAY0 + 1500, 0);
    assert_eq!(t.drain()[0].seconds, 1);
    t.active(DAY0 + 2000, 0);
    assert_eq!(t.drain()[0].seconds, 1);
}

#[test]
fn turning_forward_counts_the_page() {
    let (book, mut r) = (book(), Reading::new(DAY0));
    r.event(&book, Event::Moved, at(0, 0), DAY0, 0);
    r.event(&book, Event::Moved, at(0, 2), DAY0 + 2 * MIN, 0);
    let sums = r.tally.drain();
    assert_eq!(sums[0].chars, 2000, "two blocks of 1000 characters, not their bytes");
    assert_eq!(sums[0].seconds, 120);
}

#[test]
fn a_jump_counts_nothing_and_counting_goes_on_from_its_target() {
    let (book, mut r) = (book(), Reading::new(DAY0));
    r.event(&book, Event::Moved, at(1, 5), DAY0, 0);
    r.event(&book, Event::Jumped, at(9, 0), DAY0 + 5 * MIN, 0);
    r.event(&book, Event::Moved, at(9, 1), DAY0 + 6 * MIN, 0);
    assert_eq!(r.tally.drain()[0].chars, 1000, "only the page after chapter 10's start");
}

#[test]
fn going_back_neither_adds_nor_removes() {
    let (book, mut r) = (book(), Reading::new(DAY0));
    r.event(&book, Event::Moved, at(0, 3), DAY0, 0);
    r.event(&book, Event::Moved, at(0, 2), DAY0 + MIN, 0);
    r.event(&book, Event::Moved, at(0, 3), DAY0 + 2 * MIN, 0);
    assert_eq!(r.tally.drain()[0].chars, 1000);
}

#[test]
fn flipping_through_counts_at_most_3000_characters_a_minute() {
    let (book, mut r) = (book(), Reading::new(DAY0));
    r.event(&book, Event::Moved, at(0, 0), DAY0, 0);
    // 15 pages of 1000 characters in 20 seconds.
    for i in 1..=15 {
        r.event(&book, Event::Moved, at(i / 10, i % 10), DAY0 + i as u64 * 20_000 / 15, 0);
    }
    assert!(r.tally.drain()[0].chars <= 1000);
}

#[test]
fn the_log_is_appended_once_a_minute_or_when_leaving() {
    let mut r = Reading::new(DAY0);
    r.tally.active(DAY0, 0);
    r.tally.active(DAY0 + 20_000, 0);
    assert!(r.due(DAY0 + 20_000, false).is_empty());
    r.tally.active(DAY0 + 70_000, 0);
    assert_eq!(r.due(DAY0 + 70_000, false)[0].seconds, 70);
    r.tally.active(DAY0 + 80_000, 0);
    assert_eq!(r.due(DAY0 + 80_000, true)[0].seconds, 10);
}

#[test]
fn days_are_appended_under_the_device_and_survive_the_file() {
    let mut s = store();
    let sum = |seconds| Sum { day: "2026-10-18".into(), seconds, chars: 10, pages: 0 };
    append(&mut s, "b", &[sum(600)], DAY0).unwrap();
    append(&mut s, "b", &[sum(30)], DAY0 - 5000).unwrap(); // a clock set back
    let device = s.setting(crate::sync::DEVICE).unwrap().to_owned();
    let log = Log::read(&path(&s)).unwrap();
    let day = log.day(&device, "b", "2026-10-18").unwrap();
    assert_eq!((day.seconds, day.chars, day.time), (630, 20, DAY0 + 1), "the time still later than the line's");
    assert_eq!(log.days.len(), 1);
}

#[test]
fn damaged_lines_are_skipped_and_fields_escaped() {
    let s = store();
    let text = "forzac-log 1\nread\tb\t5\td\t2026-13-01\t1\t1\t0\nread\tb\tx\nfinish\tb\t7\tA\\tB\tC\t-\t1\nwhat\nfirst\tb\t3\n";
    std::fs::write(path(&s), text).unwrap();
    let log = Log::read(&path(&s)).unwrap();
    assert!(log.days.is_empty(), "no 13th month");
    assert_eq!(log.finishes[0].title, "A\tB");
    assert_eq!((log.finishes[0].started, log.finishes[0].approximate), (None, true));
    assert_eq!(log.first["b"], 3);
    let again = change(&s, |_| {}).unwrap();
    assert_eq!(again, log);
    assert!(std::fs::read_to_string(path(&s)).unwrap().contains("finish\tb\t7\tA\\tB\tC\t-\t1\n"));
}

#[test]
fn a_finish_is_recorded_once_within_30_days() {
    let (s, book) = (store(), book());
    opened(&s, "b", DAY0 - 12 * 86_400_000).unwrap();
    finish(&s, &book, DAY0).unwrap();
    finish(&s, &book, DAY0 + 86_400_000).unwrap();
    let log = load(&s).unwrap();
    assert_eq!(log.finishes.len(), 1);
    assert_eq!(log.finishes[0].started, Some(DAY0 - 12 * 86_400_000));
    assert_eq!((log.finishes[0].title.as_str(), log.finishes[0].authors.as_str()), ("Книга", "Автор"));
    // Read again a year later.
    finish(&s, &book, DAY0 + 365 * 86_400_000).unwrap();
    assert_eq!(load(&s).unwrap().finishes.len(), 2);
}

#[test]
fn another_devices_finish_counts_for_the_30_days() {
    let (s, book) = (store(), book());
    change(&s, |log| {
        log.add_finish(Finish {
            book: "b".into(),
            time: DAY0 - 86_400_000,
            title: "К".into(),
            authors: String::new(),
            started: None,
            approximate: false,
        })
    })
    .unwrap();
    finish(&s, &book, DAY0).unwrap();
    assert_eq!(load(&s).unwrap().finishes.len(), 1);
}

#[test]
fn the_first_open_is_kept() {
    let s = store();
    opened(&s, "b", 100).unwrap();
    opened(&s, "b", 200).unwrap();
    assert_eq!(load(&s).unwrap().first["b"], 100);
}

#[test]
fn the_first_log_finishes_the_books_read_to_their_end() {
    let dir = store().path().parent().unwrap().to_owned();
    let book_path = dir.join("book.fb2");
    std::fs::write(&book_path, "x").unwrap();
    let mut s = Store::open(dir.join("state"));
    let mut done = book();
    s.save_position(&done, &book_path, at(0, 0), true).unwrap();
    done.id = "c".into();
    s.save_position(&done, &dir.join("other.fb2"), at(0, 0), false).unwrap();
    let opened = s.entries()[0].opened;
    let log = load(&s).unwrap();
    assert_eq!(log.finishes.len(), 1);
    let f = &log.finishes[0];
    assert_eq!((f.book.as_str(), f.time, f.approximate, f.started), ("b", opened * 1000, true, None));
    assert_eq!(s.entries()[0].progress, 1.0, "the state does not change");
}

#[test]
fn dates_round_trip() {
    assert_eq!(date_string(day_number(DAY0, 0)), "2026-10-18");
    assert_eq!(date_string(day_number(DAY0 - 1, MSK)), "2026-10-18");
    assert_eq!(date_string(day_number(DAY0 - 1, 0)), "2026-10-17");
    for s in ["2024-02-29", "2026-01-01", "1999-12-31"] {
        assert_eq!(date_string(parse_date(s).unwrap()), s);
    }
    assert_eq!(parse_date("2026-02-29"), None);
    assert_eq!(parse_date("2026-1-01"), None);
}

#[test]
fn the_end_page_counts_its_book_before_the_finish_is_written() {
    let book = book();
    let log = Log::default().as_if_finished(&book, DAY0);
    assert_eq!(log.finishes.len(), 1);
    // Finished last week: not again.
    assert_eq!(log.clone().as_if_finished(&book, DAY0 + 7 * 86_400_000).finishes.len(), 1);
}
