use super::*;
use crate::i18n::{Language, set_language};
use crate::log::Day;

const DAY: u64 = 86_400_000;

fn ms(y: i64, m: u32, d: u32) -> u64 {
    days_from_civil(y, m, d) as u64 * DAY + 12 * 3_600_000
}

fn read(device: &str, book: &str, day: &str, minutes: u64) -> Day {
    Day { book: book.into(), time: 1, device: device.into(), day: day.into(), seconds: minutes * 60, chars: 0, pages: 0 }
}

fn finish(book: &str, time: u64, approximate: bool) -> Finish {
    Finish { book: book.into(), time, title: format!("Книга {book}"), authors: "Автор".into(), started: None, approximate }
}

fn oct() -> Period {
    Period::Month { year: 2026, month: 10 }
}

/// English: tests in one process share the interface language, and others expect English. The Russian texts are
/// checked in `tests/stats_russian.rs`, a process of its own.
fn english() {
    set_language(Language::English);
}

#[test]
fn a_book_read_twice_in_a_year_is_one_book_of_the_year() {
    let log = Log { finishes: vec![finish("a", ms(2026, 3, 10), false), finish("a", ms(2026, 11, 2), false)], ..Log::default() };
    assert_eq!(compute(Period::Year(2026), &log, 0).books.len(), 1);
    assert_eq!(compute(Period::Month { year: 2026, month: 3 }, &log, 0).books.len(), 1);
    assert_eq!(compute(Period::Month { year: 2026, month: 11 }, &log, 0).books.len(), 1);
    assert_eq!(compute(Period::Year(2026), &log, 0).books[0].time, ms(2026, 11, 2), "the latest finish");
}

#[test]
fn time_and_days_sum_every_book_and_device() {
    let log = Log {
        days: vec![
            read("mac", "a", "2026-10-03", 240),
            read("phone", "b", "2026-10-03", 360),
            read("phone", "a", "2026-10-04", 0),
            read("mac", "a", "2026-09-30", 50),
        ],
        finishes: vec![finish("a", ms(2026, 10, 3), false), finish("b", ms(2026, 10, 5), false)],
        ..Log::default()
    };
    let s = compute(oct(), &log, 0);
    assert_eq!(s.seconds, 600 * 60);
    assert_eq!(s.books.len(), 2);
    assert_eq!(s.books[0].book, "b", "the latest first");
    assert_eq!(s.reading_days, 1);
    english();
    assert_eq!(figures(&s)[0].0, "10 h");
}

#[test]
fn half_a_minute_on_each_device_is_a_day_with_a_book() {
    let mut a = read("mac", "a", "2026-10-03", 0);
    a.seconds = 40;
    let mut b = read("phone", "a", "2026-10-03", 0);
    b.seconds = 40;
    let s = compute(oct(), &Log { days: vec![a.clone(), b], ..Log::default() }, 0);
    assert_eq!(s.reading_days, 1);
    // Twenty seconds alone: time, but no day with a book.
    a.seconds = 20;
    let s = compute(oct(), &Log { days: vec![a], ..Log::default() }, 0);
    assert_eq!((s.seconds, s.reading_days, s.day_minutes.len()), (20, 0, 0));
}

#[test]
fn new_years_midnight_splits_between_the_years() {
    let log = Log { days: vec![read("mac", "a", "2026-12-31", 30), read("mac", "a", "2027-01-01", 30)], ..Log::default() };
    assert_eq!(compute(Period::Year(2026), &log, 0).seconds, 1800);
    assert_eq!(compute(Period::Month { year: 2027, month: 1 }, &log, 0).seconds, 1800);
}

#[test]
fn a_finish_is_dated_by_the_local_day() {
    // 23:30 UTC on 31 October is 1 November in Moscow.
    let f = finish("a", days_from_civil(2026, 10, 31) as u64 * DAY + 23 * 3_600_000 + 1_800_000, false);
    let log = Log { finishes: vec![f], ..Log::default() };
    assert_eq!(compute(oct(), &log, 0).books.len(), 1);
    assert_eq!(compute(oct(), &log, 3 * 3600).books.len(), 0);
}

#[test]
fn finished_offline_on_two_devices_counts_once() {
    let log = Log { finishes: vec![finish("a", ms(2026, 10, 31), false), finish("a", ms(2026, 11, 2), false)], ..Log::default() };
    assert_eq!(compute(oct(), &log, 0).books.len(), 1);
    assert_eq!(compute(Period::Month { year: 2026, month: 11 }, &log, 0).books.len(), 0);
    assert_eq!(compute(Period::Year(2026), &log, 0).books.len(), 1);
}

#[test]
fn approximate_dates_from_two_devices_count_once_the_earlier() {
    let log = Log { finishes: vec![finish("a", ms(2026, 10, 20), true), finish("a", ms(2025, 5, 4), true)], ..Log::default() };
    assert_eq!(compute(Period::Month { year: 2025, month: 5 }, &log, 0).books.len(), 1);
    assert_eq!(compute(oct(), &log, 0).books.len(), 0);
}

#[test]
fn an_approximate_finish_before_an_exact_one_gives_way() {
    let log = Log { finishes: vec![finish("a", ms(2026, 10, 20), true), finish("a", ms(2026, 11, 3), false)], ..Log::default() };
    assert_eq!(compute(oct(), &log, 0).books.len(), 0);
    assert_eq!(compute(Period::Month { year: 2026, month: 11 }, &log, 0).books.len(), 1);
}

#[test]
fn levels() {
    assert_eq!([0, 1, 14, 15, 29, 30, 59, 60, 300].map(level), [0, 1, 1, 2, 2, 3, 3, 4, 4]);
}

#[test]
fn time_reads_in_minutes_then_whole_hours() {
    english();
    assert_eq!(
        [0, 59, 35 * 60, 3599, 3600, 6 * 3600 + 10 * 60, 41 * 3600 + 31 * 60].map(duration),
        ["0 min", "0 min", "35 min", "59 min", "1 h", "6 h", "42 h"]
    );
}

#[test]
fn periods_step_and_name_themselves() {
    assert_eq!(Period::Month { year: 2026, month: 1 }.step(-1), Period::Month { year: 2025, month: 12 });
    assert_eq!(Period::Month { year: 2026, month: 12 }.step(1), Period::Month { year: 2027, month: 1 });
    assert_eq!(Period::Year(2026).step(-1), Period::Year(2025));
    assert_eq!(oct().days(), (days_from_civil(2026, 10, 1), days_from_civil(2026, 10, 31)));
    assert_eq!(Period::Month { year: 2024, month: 2 }.days().1, days_from_civil(2024, 2, 29));
    assert_eq!(oct().file_name(), "forzac-2026-10.png");
    assert_eq!(Period::Year(2026).file_name(), "forzac-2026.png");
}

#[test]
fn a_months_grid_by_the_weeks_start() {
    // October 2026 begins on a Thursday.
    let monday = MonthGrid::new(2026, 10, 0);
    assert_eq!((monday.offset, monday.weeks), (3, 5));
    assert_eq!(monday.cell(1), (0, 3));
    assert_eq!(monday.cell(31), (4, 5));
    let sunday = MonthGrid::new(2026, 10, 6);
    assert_eq!(sunday.offset, 4);
    // February 2027 from a Monday: four full weeks.
    assert_eq!(MonthGrid::new(2027, 2, 0).weeks, 4);
    // August 2026 from a Monday: starts on a Saturday, six weeks.
    assert_eq!(MonthGrid::new(2026, 8, 0).weeks, 6);
}

#[test]
fn the_calendar_of_a_month_and_a_year() {
    english();
    let month = calendar(oct(), 7.0 * 40.0 + 6.0 * 6.0, 1.0, 3);
    assert_eq!(month.labels.iter().map(|l| l.text.as_str()).collect::<Vec<_>>(), ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"]);
    assert_eq!(month.cells.len(), 31);
    assert_eq!((month.cells[0].rect.x, month.cells[0].rect.y, month.cells[0].rect.w), (4.0 * 46.0, 22.0, 40.0));
    assert_eq!(month.height, 22.0 + 5.0 * 40.0 + 4.0 * 6.0);
    assert!(month.numbers);
    let year = calendar(Period::Year(2026), 904.0, 2.5, 4);
    assert_eq!(year.cells.len(), 365);
    assert_eq!(year.labels.len(), 12);
    assert_eq!(year.labels[4].text, "May");
    assert!(year.labels[4].rect.x == 0.0 && year.labels[4].rect.y > 0.0, "four months a row");
    assert!(!year.numbers);
}

#[test]
fn the_window_view() {
    english();
    let log = Log {
        days: vec![read("mac", "a", "2026-09-30", 50), read("mac", "a", "2026-10-12", 48)],
        finishes: vec![finish("a", ms(2026, 10, 12), false)],
        ..Log::default()
    };
    let now = ms(2026, 10, 18);
    let v = view(&log, oct(), now, 0, 300.0, 1.0, 3);
    assert_eq!(v.title, "October 2026");
    assert!(v.can_previous && !v.can_next);
    assert!(!view(&log, Period::Month { year: 2026, month: 9 }, now, 0, 300.0, 1.0, 3).can_previous);
    let twelfth = v.cells.iter().find(|c| c.number == Some(12)).unwrap();
    assert_eq!((twelfth.level, twelfth.minutes), (3, 48));
    assert_eq!(day_line(twelfth.day, twelfth.minutes), "12 October · 48 min");
    assert!(v.cells.iter().find(|c| c.number == Some(18)).unwrap().today);
    assert!(v.cells.iter().filter(|c| c.future).count() == 13);
    assert_eq!(v.books, [("Книга a".to_owned(), "Автор · 12 October".to_owned())]);
    assert_eq!(v.figures[1], ("1".to_owned(), "book finished".to_owned()));
    // An empty log: only this month.
    let empty = view(&Log::default(), oct(), now, 0, 300.0, 1.0, 3);
    assert!(!empty.can_previous && !empty.can_next && empty.books.is_empty());
}

#[test]
fn the_end_pages_lines() {
    english();
    let now = ms(2026, 10, 18);
    let mut log = Log {
        days: vec![read("mac", "a", "2026-10-07", 250), read("phone", "a", "2026-10-08", 120), read("mac", "b", "2026-10-09", 470)],
        finishes: vec![finish("a", now, false), finish("c", ms(2026, 10, 2), false)],
        ..Log::default()
    };
    log.first.insert("a".into(), ms(2026, 10, 7));
    assert_eq!(book_line(&log, "a", 7, 0, now, 0), "6 h · 12 days · 7 highlights");
    assert_eq!(month_line(&log, now, 0), "October: 2 books · 14 h");
    // A book first opened before the log began: no time.
    log.first.insert("a".into(), ms(2026, 10, 6));
    assert_eq!(book_line(&log, "a", 0, 2, now, 0), "13 days · 2 bookmarks");
    // Not known when it was first opened: neither time nor days.
    assert_eq!(book_line(&log, "x", 1, 0, now, 0), "1 highlight");
}
