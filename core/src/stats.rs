//! Reading statistics (`reading-stats`) from the reading log of every device (`log`): a month's or a year's time, days
//! with a book and finished books, the reading calendar's geometry, and the texts of the stats window, the picture and
//! the end page — the Android app's `StatsCalculator`, `CalendarGrid` and `CalendarLayout`, in the interface language.

use std::collections::{BTreeMap, HashMap};

use crate::i18n::{Language, fill, language, tr, tr_count};
use crate::layout::Rect;
use crate::log::{Finish, Log, SAME_READING, day_number, days_from_civil, days_in_month, parse_date};

/// A calendar month or year.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Period {
    Month { year: i64, month: u32 },
    Year(i64),
}

impl Period {
    /// The month of the day number `day`.
    pub fn month_of(day: i64) -> Period {
        let (year, month, _) = crate::sync::civil(day);
        Period::Month { year, month }
    }

    /// The first and the last day numbers.
    pub fn days(self) -> (i64, i64) {
        match self {
            Period::Month { year, month } => {
                let first = days_from_civil(year, month, 1);
                (first, first + i64::from(days_in_month(year, month)) - 1)
            }
            Period::Year(year) => (days_from_civil(year, 1, 1), days_from_civil(year, 12, 31)),
        }
    }

    /// The period before (`-1`) or after (`1`).
    pub fn step(self, delta: i64) -> Period {
        match self {
            Period::Month { year, month } => {
                let n = year * 12 + i64::from(month) - 1 + delta;
                Period::Month { year: n.div_euclid(12), month: n.rem_euclid(12) as u32 + 1 }
            }
            Period::Year(year) => Period::Year(year + delta),
        }
    }

    /// The month or the year holding `day`, as this period is.
    pub fn like(self, day: i64) -> Period {
        match (self, Period::month_of(day)) {
            (Period::Year(_), Period::Month { year, .. }) => Period::Year(year),
            (_, month) => month,
        }
    }

    /// "October 2026" or "2026".
    pub fn title(self) -> String {
        match self {
            Period::Month { year, month } => format!("{} {year}", month_name(month)),
            Period::Year(year) => year.to_string(),
        }
    }

    /// For the picture: the month's name and its year, or the year alone.
    pub fn big_and_small(self) -> (String, String) {
        match self {
            Period::Month { year, month } => (month_name(month).to_owned(), year.to_string()),
            Period::Year(year) => (year.to_string(), String::new()),
        }
    }

    /// The picture's file name: `forzac-2026-10.png` or `forzac-2026.png`.
    pub fn file_name(self) -> String {
        match self {
            Period::Month { year, month } => format!("forzac-{year}-{month:02}.png"),
            Period::Year(year) => format!("forzac-{year}.png"),
        }
    }
}

/// A period's numbers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Stats {
    pub seconds: u64,
    /// Days with at least a minute read.
    pub reading_days: usize,
    /// The counted finishes in the period, one per book, the latest first.
    pub books: Vec<Finish>,
    /// Minutes by day number, for the days with at least a minute.
    pub day_minutes: BTreeMap<i64, u64>,
}

/// The finishes that count: one reading marked by several finishes counts once. Going through a book's finishes by
/// time, one less than 30 days after a counted one does not count, and an approximate one does not count when the book
/// has an earlier finish or an exact one less than 30 days after it (shared with the Android app's
/// `sync-reading-log`).
pub fn counted(finishes: &[Finish]) -> Vec<&Finish> {
    let mut by_book: HashMap<&str, Vec<&Finish>> = HashMap::new();
    for f in finishes {
        by_book.entry(&f.book).or_default().push(f);
    }
    let mut out = Vec::new();
    for list in by_book.values_mut() {
        list.sort_by_key(|f| f.time);
        let mut kept: Vec<u64> = Vec::new();
        for (i, f) in list.iter().enumerate() {
            if kept.iter().any(|&t| f.time < t + SAME_READING) {
                continue;
            }
            let earlier = list[..i].iter().any(|o| o.time < f.time);
            let exact_soon = list[i + 1..].iter().any(|o| !o.approximate && o.time > f.time && o.time < f.time + SAME_READING);
            if f.approximate && (earlier || exact_soon) {
                continue;
            }
            kept.push(f.time);
            out.push(*f);
        }
    }
    out
}

/// The numbers of `period` from `log`, local dates in the zone `offset` seconds east of UTC.
pub fn compute(period: Period, log: &Log, offset: i64) -> Stats {
    let (first, last) = period.days();
    let mut seconds_by_day: BTreeMap<i64, u64> = BTreeMap::new();
    for d in &log.days {
        if let Some(day) = parse_date(&d.day).filter(|day| (first..=last).contains(day)) {
            *seconds_by_day.entry(day).or_default() += d.seconds;
        }
    }
    let mut books: Vec<Finish> =
        counted(&log.finishes).into_iter().filter(|f| (first..=last).contains(&day_number(f.time, offset))).cloned().collect();
    books.sort_by_key(|f| std::cmp::Reverse(f.time));
    let mut seen = std::collections::HashSet::new();
    books.retain(|f| seen.insert(f.book.clone()));
    let day_minutes: BTreeMap<i64, u64> = seconds_by_day.iter().map(|(&d, &s)| (d, s / 60)).filter(|&(_, m)| m >= 1).collect();
    Stats { seconds: seconds_by_day.values().sum(), reading_days: day_minutes.len(), books, day_minutes }
}

/// A day's shade: 0 under a minute, 1 to 14 minutes, 15 to 29, 30 to 59, an hour and more.
pub fn level(minutes: u64) -> u8 {
    match minutes {
        0 => 0,
        1..=14 => 1,
        15..=29 => 2,
        30..=59 => 3,
        _ => 4,
    }
}

/// The first day with a log line or a finish; none when the log is empty.
pub fn first_day(log: &Log, offset: i64) -> Option<i64> {
    log.days.iter().filter_map(|d| parse_date(&d.day)).chain(log.finishes.iter().map(|f| day_number(f.time, offset))).min()
}

// region Texts

const MONTHS_EN: [&str; 12] =
    ["January", "February", "March", "April", "May", "June", "July", "August", "September", "October", "November", "December"];
const MONTHS_RU: [&str; 12] =
    ["Январь", "Февраль", "Март", "Апрель", "Май", "Июнь", "Июль", "Август", "Сентябрь", "Октябрь", "Ноябрь", "Декабрь"];
/// Short weekday names from Monday.
const WEEKDAYS_EN: [&str; 7] = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
const WEEKDAYS_RU: [&str; 7] = ["пн", "вт", "ср", "чт", "пт", "сб", "вс"];

/// The month's name for a title: "October", "Октябрь".
pub fn month_name(month: u32) -> &'static str {
    let names = if language() == Language::Russian { &MONTHS_RU } else { &MONTHS_EN };
    names[(month as usize).clamp(1, 12) - 1]
}

/// A date in a line: "18 October", "18 октября".
pub fn date(day: i64) -> String {
    let (_, month, d) = crate::sync::civil(day);
    // The catalogue has the months' Russian genitive (`sync::when`).
    let months = [
        tr("January"),
        tr("February"),
        tr("March"),
        tr("April"),
        tr("May"),
        tr("June"),
        tr("July"),
        tr("August"),
        tr("September"),
        tr("October"),
        tr("November"),
        tr("December"),
    ];
    format!("{d} {}", months[month as usize - 1])
}

/// Time read: minutes under an hour ("35 min"), else whole hours, rounded ("8 h").
pub fn duration(seconds: u64) -> String {
    let minutes = seconds / 60;
    if minutes < 60 {
        fill(tr("{n} min"), &[("n", &minutes.to_string())])
    } else {
        fill(tr("{n} h"), &[("n", &((minutes + 30) / 60).to_string())])
    }
}

/// The three figures with their captions: time read, books finished, days with a book.
pub fn figures(stats: &Stats) -> [(String, String); 3] {
    let (books, days) = (stats.books.len(), stats.reading_days);
    [
        (duration(stats.seconds), tr("reading").to_owned()),
        (books.to_string(), tr_count(books, "book finished", "books finished")),
        (days.to_string(), tr_count(days, "day with a book", "days with a book")),
    ]
}

/// The line under the calendar for a clicked day: "18 October · 52 min" or "18 October · not read".
pub fn day_line(day: i64, minutes: u64) -> String {
    let time = if minutes == 0 { tr("not read").to_owned() } else { duration(minutes * 60) };
    format!("{} · {time}", date(day))
}

/// A finished book's second line: "Alexander Pushkin · 26 September".
pub fn book_caption(f: &Finish, offset: i64) -> String {
    let date = date(day_number(f.time, offset));
    if f.authors.trim().is_empty() { date } else { format!("{} · {date}", f.authors) }
}

/// The end page's month: "October: 3 books · 14 h" for the month of `now`.
pub fn month_line(log: &Log, now: u64, offset: i64) -> String {
    let period = Period::month_of(day_number(now, offset));
    let stats = compute(period, log, offset);
    let Period::Month { month, .. } = period else { unreachable!() };
    let books = tr_count(stats.books.len(), "{n} book", "{n} books");
    format!("{}: {books} · {}", month_name(month), duration(stats.seconds))
}

/// The end page's totals of the book `id`, the parts that are not zero: its time by the log of every device (only when
/// the log began by the day it was first opened here, `first`), the days since then, its highlights and bookmarks.
pub fn book_line(log: &Log, id: &str, highlights: usize, bookmarks: usize, now: u64, offset: i64) -> String {
    let mut parts = Vec::new();
    let first = log.first.get(id).map(|&t| day_number(t, offset));
    let start = log.days.iter().filter_map(|d| parse_date(&d.day)).min();
    if let (Some(first), Some(start)) = (first, start)
        && start <= first
    {
        let seconds: u64 = log.days.iter().filter(|d| d.book == id).map(|d| d.seconds).sum();
        if seconds >= 60 {
            parts.push(duration(seconds));
        }
    }
    if let Some(first) = first {
        let days = (day_number(now, offset) - first + 1).max(1) as usize;
        parts.push(tr_count(days, "{n} day", "{n} days"));
    }
    if highlights > 0 {
        parts.push(tr_count(highlights, "{n} highlight", "{n} highlights"));
    }
    if bookmarks > 0 {
        parts.push(tr_count(bookmarks, "{n} bookmark", "{n} bookmarks"));
    }
    parts.join(" · ")
}

// endregion

// region The calendar

/// The weekday of a day number, Monday 0.
fn weekday(day: i64) -> i64 {
    (day + 3).rem_euclid(7)
}

/// The week starts on Monday in Russian, on Sunday in English: the Monday-0 weekday it starts on.
pub fn week_start() -> i64 {
    if language() == Language::Russian { 0 } else { 6 }
}

/// Where a month's days go: empty cells before the 1st, and its weeks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MonthGrid {
    pub first: i64,
    pub length: i64,
    pub offset: i64,
    pub weeks: i64,
}

impl MonthGrid {
    pub fn new(year: i64, month: u32, start: i64) -> MonthGrid {
        let (first, length) = (days_from_civil(year, month, 1), i64::from(days_in_month(year, month)));
        let offset = (weekday(first) - start).rem_euclid(7);
        MonthGrid { first, length, offset, weeks: (offset + length + 6) / 7 }
    }

    /// The row and column of the month's day `n` (1-based).
    pub fn cell(&self, n: i64) -> (i64, i64) {
        let i = self.offset + n - 1;
        (i / 7, i % 7)
    }
}

/// A day's cell.
#[derive(Debug, Clone, PartialEq)]
pub struct Cell {
    pub day: i64,
    pub rect: Rect,
}

/// A weekday's or a month's name over the cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub text: String,
    pub rect: Rect,
    pub centered: bool,
}

/// The reading calendar laid out in points (times `scale`) from the top left.
#[derive(Debug, Clone, PartialEq)]
pub struct Calendar {
    pub height: f32,
    pub cells: Vec<Cell>,
    pub labels: Vec<Label>,
    pub radius: f32,
    /// A month's cells hold their day numbers; a year's do not.
    pub numbers: bool,
}

/// The calendar of `period` `width` wide: a month in seven columns with the weekdays above, or a year of twelve
/// months `columns` a row (3 in the window, 4 in the picture).
pub fn calendar(period: Period, width: f32, scale: f32, columns: usize) -> Calendar {
    let start = week_start();
    match period {
        Period::Month { year, month } => {
            let grid = MonthGrid::new(year, month, start);
            let gap = 6.0 * scale;
            let cell = (width - 6.0 * gap) / 7.0;
            let header = 22.0 * scale;
            let names = if language() == Language::Russian { &WEEKDAYS_RU } else { &WEEKDAYS_EN };
            let labels = (0..7)
                .map(|c| Label {
                    text: names[((start + c) % 7) as usize].to_owned(),
                    rect: Rect { x: c as f32 * (cell + gap), y: 0.0, w: cell, h: header },
                    centered: true,
                })
                .collect();
            let cells = (1..=grid.length)
                .map(|n| {
                    let (row, column) = grid.cell(n);
                    let rect = Rect { x: column as f32 * (cell + gap), y: header + row as f32 * (cell + gap), w: cell, h: cell };
                    Cell { day: grid.first + n - 1, rect }
                })
                .collect();
            let weeks = grid.weeks as f32;
            Calendar { height: header + weeks * cell + (weeks - 1.0) * gap, cells, labels, radius: 8.0 * scale, numbers: true }
        }
        Period::Year(year) => {
            let columns = columns.max(1);
            let (column_gap, row_gap, gap) = (20.0 * scale, 18.0 * scale, 2.0 * scale);
            let month_width = (width - (columns - 1) as f32 * column_gap) / columns as f32;
            let cell = (month_width - 6.0 * gap) / 7.0;
            let header = 20.0 * scale;
            // Six weeks for every month: the rows of months do not jump in height.
            let block = header + 6.0 * cell + 5.0 * gap;
            let (mut cells, mut labels) = (Vec::with_capacity(366), Vec::with_capacity(12));
            for m in 1..=12u32 {
                let grid = MonthGrid::new(year, m, start);
                let x = ((m as usize - 1) % columns) as f32 * (month_width + column_gap);
                let y = ((m as usize - 1) / columns) as f32 * (block + row_gap);
                labels.push(Label { text: month_name(m).to_owned(), rect: Rect { x, y, w: month_width, h: header }, centered: false });
                for n in 1..=grid.length {
                    let (row, column) = grid.cell(n);
                    let rect = Rect { x: x + column as f32 * (cell + gap), y: y + header + row as f32 * (cell + gap), w: cell, h: cell };
                    cells.push(Cell { day: grid.first + n - 1, rect });
                }
            }
            let rows = 12usize.div_ceil(columns) as f32;
            Calendar { height: rows * block + (rows - 1.0) * row_gap, cells, labels, radius: 2.0 * scale, numbers: false }
        }
    }
}

// endregion

// region What the window shows

/// A cell as drawn: its day, rectangle, shade, today's outline, a future day (not shaded), its number and its line.
#[derive(Debug, Clone, PartialEq)]
pub struct DayCell {
    pub day: i64,
    pub rect: Rect,
    pub level: u8,
    pub today: bool,
    pub future: bool,
    /// The day of the month, when the calendar shows numbers.
    pub number: Option<u32>,
    pub minutes: u64,
}

/// Everything the stats window (and its picture) shows for one period.
#[derive(Debug, Clone, PartialEq)]
pub struct View {
    pub period: Period,
    pub title: String,
    pub figures: [(String, String); 3],
    pub can_previous: bool,
    pub can_next: bool,
    pub height: f32,
    pub radius: f32,
    pub cells: Vec<DayCell>,
    pub labels: Vec<Label>,
    /// Title and "authors · date" of each finished book, the latest first.
    pub books: Vec<(String, String)>,
}

/// The stats window's `period` from `log` as of `now` (milliseconds) in the zone `offset`; the calendar `width` points
/// wide at `scale`, a year `columns` months a row.
pub fn view(log: &Log, period: Period, now: u64, offset: i64, width: f32, scale: f32, columns: usize) -> View {
    let today = day_number(now, offset);
    let stats = compute(period, log, offset);
    let first = first_day(log, offset).unwrap_or(today).min(today);
    let calendar = calendar(period, width, scale, columns);
    let cells = calendar
        .cells
        .iter()
        .map(|c| {
            let minutes = stats.day_minutes.get(&c.day).copied().unwrap_or(0);
            let future = c.day > today;
            DayCell {
                day: c.day,
                rect: c.rect,
                level: if future { 0 } else { level(minutes) },
                today: c.day == today,
                future,
                number: calendar.numbers.then(|| crate::sync::civil(c.day).2),
                minutes,
            }
        })
        .collect();
    let (start, end) = period.days();
    View {
        period,
        title: period.title(),
        figures: figures(&stats),
        can_previous: start > period.like(first).days().0,
        can_next: end < today,
        height: calendar.height,
        radius: calendar.radius,
        cells,
        labels: calendar.labels,
        books: stats.books.iter().map(|f| (f.title.clone(), book_caption(f, offset))).collect(),
    }
}

// endregion

#[cfg(test)]
mod tests;
