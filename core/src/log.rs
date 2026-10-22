//! The reading log (`reading-log`): how long and how much each book was read on each local day, on this computer and,
//! through sync, on the others; when books were finished; when they were first opened here. One text file beside the
//! reading state, written as the state is (read again under a lock, changed, replaced):
//!
//! ```text
//! forzac-log 1
//! read<TAB>book<TAB>time<TAB>device<TAB>day<TAB>seconds<TAB>chars<TAB>pages
//! finish<TAB>book<TAB>time<TAB>title<TAB>authors<TAB>started<TAB>approximate
//! first<TAB>book<TAB>time
//! ```
//!
//! `read` and `finish` are the sync file's lines (`docs/sync-format.md`); `first` stays here. Times are milliseconds
//! since the Unix epoch, `day` is the local date `yyyy-mm-dd`, `started` a time or `-`, `approximate` `1` or `0`.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::model::{Book, Locator};
use crate::state::{Store, escape, unescape};

/// The log's file name, beside the state.
pub const FILE: &str = "reading-log";
const HEADER: &str = "forzac-log 1";
/// A gap between two actions counts up to this.
const MAX_GAP: u64 = 5 * 60_000;
/// Three times an average reader's speed: faster is flipping through.
const CHARS_PER_MINUTE: u64 = 3000;
/// A book finished again within this is the same reading.
pub const SAME_READING: u64 = 30 * 86_400_000;
/// The log is appended at most this often while reading.
const APPEND_EVERY: u64 = 60_000;
const DAY_MS: i64 = 86_400_000;

/// A day of a device's reading of a book.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Day {
    pub book: String,
    /// When the line last changed.
    pub time: u64,
    pub device: String,
    /// `yyyy-mm-dd`, local to the device.
    pub day: String,
    pub seconds: u64,
    pub chars: u64,
    pub pages: u64,
}

/// A book finished.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finish {
    pub book: String,
    /// When it was finished.
    pub time: u64,
    pub title: String,
    pub authors: String,
    /// When it was first opened on the device that finished it.
    pub started: Option<u64>,
    /// Dated by its last opening when the log began, not seen finished.
    pub approximate: bool,
}

/// What the log holds.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Log {
    pub days: Vec<Day>,
    pub finishes: Vec<Finish>,
    /// When each book was first opened on this computer.
    pub first: BTreeMap<String, u64>,
}

/// A line of the log or of a sync file.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Line {
    Read(Day),
    Finish(Finish),
    First(String, u64),
}

/// `read` or `finish` as a line, without the line break.
pub(crate) fn day_line(d: &Day) -> String {
    format!("read\t{}\t{}\t{}\t{}\t{}\t{}\t{}", escape(&d.book), d.time, escape(&d.device), d.day, d.seconds, d.chars, d.pages)
}

pub(crate) fn finish_line(f: &Finish) -> String {
    let started = f.started.map_or("-".to_owned(), |s| s.to_string());
    let (book, title, authors) = (escape(&f.book), escape(&f.title), escape(&f.authors));
    format!("finish\t{book}\t{}\t{title}\t{authors}\t{started}\t{}", f.time, u8::from(f.approximate))
}

/// A line's fields (unescaped) as a `read`, `finish` or `first` line; extra fields are ignored.
pub(crate) fn parse_line(f: &[String]) -> Option<Line> {
    let num = |i: usize| f.get(i)?.parse::<u64>().ok();
    match f.first()?.as_str() {
        "read" => Some(Line::Read(Day {
            book: f.get(1)?.clone(),
            time: num(2)?,
            device: f.get(3)?.clone(),
            day: f.get(4).filter(|d| parse_date(d).is_some())?.clone(),
            seconds: num(5)?,
            chars: num(6)?,
            pages: num(7)?,
        })),
        "finish" => Some(Line::Finish(Finish {
            book: f.get(1)?.clone(),
            time: num(2)?,
            title: f.get(3)?.clone(),
            authors: f.get(4)?.clone(),
            started: match f.get(5)?.as_str() {
                "-" => None,
                s => Some(s.parse().ok()?),
            },
            approximate: match f.get(6)?.as_str() {
                "1" => true,
                "0" => false,
                _ => return None,
            },
        })),
        "first" => Some(Line::First(f.get(1)?.clone(), num(2)?)),
        _ => None,
    }
}

impl Log {
    /// The log at `path`; none when the file is missing or not a log.
    pub fn read(path: &Path) -> Option<Log> {
        let text = std::fs::read_to_string(path).ok()?;
        let mut lines = text.lines();
        if lines.next() != Some(HEADER) {
            return None;
        }
        let mut log = Log::default();
        for line in lines {
            let f: Vec<String> = line.split('\t').map(unescape).collect();
            match parse_line(&f) {
                Some(Line::Read(d)) => log.set_day(d),
                Some(Line::Finish(f)) => log.add_finish(f),
                Some(Line::First(book, time)) => {
                    log.first.entry(book).or_insert(time);
                }
                None => {} // unknown or damaged: dropped at the next write
            }
        }
        Some(log)
    }

    fn text(&self) -> String {
        let mut text = String::with_capacity(64 + self.days.len() * 80 + self.finishes.len() * 120);
        let _ = writeln!(text, "{HEADER}");
        for d in &self.days {
            let _ = writeln!(text, "{}", day_line(d));
        }
        for f in &self.finishes {
            let _ = writeln!(text, "{}", finish_line(f));
        }
        for (book, time) in &self.first {
            let _ = writeln!(text, "first\t{}\t{time}", escape(book));
        }
        text
    }

    /// The day of `d`'s device, book and date replaced by `d`.
    pub(crate) fn set_day(&mut self, d: Day) {
        match self.days.iter_mut().find(|o| o.device == d.device && o.book == d.book && o.day == d.day) {
            Some(o) => *o = d,
            None => self.days.push(d),
        }
    }

    /// `f` added unless the log has the book's finish of that time.
    pub(crate) fn add_finish(&mut self, f: Finish) {
        if !self.finishes.iter().any(|o| o.book == f.book && o.time == f.time) {
            self.finishes.push(f);
        }
    }

    /// Every `read` and `finish` line.
    pub(crate) fn lines(&self) -> Vec<Line> {
        self.days.iter().cloned().map(Line::Read).chain(self.finishes.iter().cloned().map(Line::Finish)).collect()
    }

    /// The log as it will be once `book`, whose end page shows, is finished at `now` (`finish`): for the end page's
    /// month, which counts the book before its finish is written.
    pub fn as_if_finished(mut self, book: &Book, now: u64) -> Log {
        if !self.finishes.iter().any(|f| f.book == book.id && f.time + SAME_READING > now) {
            let authors = book.authors.join(", ");
            self.add_finish(Finish {
                book: book.id.clone(),
                time: now,
                title: book.title.clone(),
                authors,
                started: None,
                approximate: false,
            });
        }
        self
    }

    /// The day of `device`, `book` and `day`.
    pub fn day(&self, device: &str, book: &str, day: &str) -> Option<&Day> {
        self.days.iter().find(|d| d.device == device && d.book == book && d.day == day)
    }
}

/// The log's path for the state `store`.
pub fn path(store: &Store) -> PathBuf {
    store.path().with_file_name(FILE)
}

/// The log of `store`'s computer, made with the first log's finishes when there is none yet.
pub fn load(store: &Store) -> io::Result<Log> {
    match Log::read(&path(store)) {
        Some(log) => Ok(log),
        None => change(store, |_| {}),
    }
}

/// The first log: an approximate finish for every book of the state read to its end, dated its last opening.
fn first_log(store: &Store) -> Log {
    let mut log = Log::default();
    for e in store.entries().iter().filter(|e| e.progress >= crate::library::FINISHED) {
        let edits = store.edits_of(&e.id);
        log.add_finish(Finish {
            book: e.id.clone(),
            time: e.opened * 1000,
            title: edits.title.unwrap_or_else(|| e.title.clone()),
            authors: edits.authors.unwrap_or_else(|| e.authors.clone()),
            started: None,
            approximate: true,
        });
    }
    log
}

/// Reads the log again under its lock (or makes the first one), applies `change`, replaces the file; returns the log.
pub(crate) fn change(store: &Store, change: impl FnOnce(&mut Log)) -> io::Result<Log> {
    let path = path(store);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let lock = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(path.with_file_name(format!(".{FILE}.lock")))?;
    lock.lock()?;
    let mut log = Log::read(&path).unwrap_or_else(|| first_log(store));
    change(&mut log);
    static WRITES: AtomicUsize = AtomicUsize::new(0);
    let tmp = path.with_file_name(format!(".{FILE}.{}.{}.tmp", std::process::id(), WRITES.fetch_add(1, Ordering::Relaxed)));
    let written = std::fs::write(&tmp, log.text()).and_then(|_| std::fs::rename(&tmp, &path));
    if written.is_err() {
        let _ = std::fs::remove_file(&tmp);
    }
    written?;
    Ok(log)
}

/// Adds `sums` (from [`Tally::drain`]) to this computer's days of the book `book`, as of `now`.
pub fn append(store: &mut Store, book: &str, sums: &[Sum], now: u64) -> io::Result<()> {
    if sums.is_empty() {
        return Ok(());
    }
    let device = crate::sync::device(store)?;
    change(store, |log| {
        for s in sums {
            let day = match log.day(&device, book, &s.day) {
                Some(o) => Day {
                    time: now.max(o.time + 1),
                    seconds: o.seconds + s.seconds,
                    chars: o.chars + s.chars,
                    pages: o.pages + s.pages,
                    ..o.clone()
                },
                None => Day {
                    book: book.to_owned(),
                    time: now,
                    device: device.clone(),
                    day: s.day.clone(),
                    seconds: s.seconds,
                    chars: s.chars,
                    pages: s.pages,
                },
            };
            log.set_day(day);
        }
    })
    .map(drop)
}

/// Records that `book` (its edited title and authors, if any) was finished at `now`, unless it was finished less than
/// 30 days before on any device.
pub fn finish(store: &Store, book: &Book, now: u64) -> io::Result<()> {
    let recent = |log: &Log| log.finishes.iter().any(|f| f.book == book.id && f.time + SAME_READING > now);
    if load(store).is_ok_and(|log| recent(&log)) {
        return Ok(());
    }
    let edits = store.edits_of(&book.id);
    change(store, |log| {
        if !recent(log) {
            log.add_finish(Finish {
                book: book.id.clone(),
                time: now,
                title: edits.title.unwrap_or_else(|| book.title.clone()),
                authors: edits.authors.unwrap_or_else(|| book.authors.join(", ")),
                started: log.first.get(&book.id).copied(),
                approximate: false,
            });
        }
    })
    .map(drop)
}

/// Remembers that the book `id` was first opened on this computer at `now`, unless it has been opened before.
pub fn opened(store: &Store, id: &str, now: u64) -> io::Result<()> {
    if load(store)?.first.contains_key(id) {
        return Ok(());
    }
    change(store, |log| {
        log.first.entry(id.to_owned()).or_insert(now);
    })
    .map(drop)
}

// region Counting

/// What a day's reading adds to the log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sum {
    pub day: String,
    pub seconds: u64,
    pub chars: u64,
    pub pages: u64,
}

#[derive(Debug, Clone, Copy, Default)]
struct Sums {
    millis: u64,
    chars: u64,
}

/// Counts the reading of an open book from the reader's events, each with its time (milliseconds) and the local time
/// zone (`offset`, seconds east of UTC); sums by local day until [`Tally::drain`]. The Android app's `ReadingTally`.
///
/// Time: the gap since the last event counts, at most [`MAX_GAP`]; after [`Tally::paused`] the next gap does not.
/// Characters: a forward move counts the text between the places, at most [`CHARS_PER_MINUTE`] a minute of the time
/// counted since the last counted move; going back or a jump only moves the anchor.
#[derive(Debug, Clone, Default)]
pub struct Tally {
    /// By local day number (days since 1970-01-01).
    days: BTreeMap<i64, Sums>,
    /// The last event's time; none when reading has just begun or come back after a pause.
    last: Option<u64>,
    /// The place counted from, in characters.
    anchor: Option<u64>,
    /// Time counted since the last counted move: the allowance for the speed limit.
    credit: u64,
}

impl Tally {
    /// A key, a click, a scroll step, a turn.
    pub fn active(&mut self, now: u64, offset: i64) {
        let from = self.last.replace(now);
        let Some(from) = from else { return };
        let gap = now.saturating_sub(from).min(MAX_GAP);
        self.credit += gap;
        // The gap split at local midnights.
        let (mut start, end) = (from, from + gap);
        while start < end {
            let day = day_number(start, offset);
            let midnight = ((day + 1) * DAY_MS - offset * 1000) as u64;
            let until = end.min(midnight);
            self.days.entry(day).or_default().millis += until - start;
            start = until;
        }
    }

    /// The reader moved forward or back by turning or scrolling to `chars` characters into the book.
    pub fn moved(&mut self, now: u64, offset: i64, chars: u64) {
        self.active(now, offset);
        let from = self.anchor.replace(chars);
        let Some(from) = from.filter(|&f| chars > f) else { return };
        self.days.entry(day_number(now, offset)).or_default().chars += (chars - from).min(self.credit * CHARS_PER_MINUTE / 60_000);
        self.credit = 0;
    }

    /// A jump (contents, link, search, bookmark, highlight, chapter keys, back, slider): counting goes on from `chars`.
    pub fn jumped(&mut self, chars: u64) {
        self.anchor = Some(chars);
    }

    /// The reader left: the time up to `now` counts, the next gap does not.
    pub fn paused(&mut self, now: u64, offset: i64) {
        self.active(now, offset);
        self.last = None;
    }

    /// What was counted since the last drain, by day; a fraction of a second stays for the next.
    pub fn drain(&mut self) -> Vec<Sum> {
        let rest = self.days.last_key_value().map(|(&day, s)| (day, s.millis % 1000));
        let sums = std::mem::take(&mut self.days)
            .into_iter()
            .map(|(day, s)| Sum { day: date_string(day), seconds: s.millis / 1000, chars: s.chars, pages: 0 })
            .filter(|s| s.seconds > 0 || s.chars > 0)
            .collect();
        if let Some((day, millis)) = rest.filter(|r| r.1 > 0) {
            self.days.insert(day, Sums { millis, chars: 0 });
        }
        sums
    }
}

/// What a reader's event is (`Reading::event`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    Active,
    Moved,
    Jumped,
    Paused,
}

/// The reading of an open book: its tally, the book's block starts in characters, when it was last appended.
#[derive(Debug, Default)]
pub struct Reading {
    pub tally: Tally,
    /// Characters before each block, by chapter; made at the first move.
    starts: Vec<Vec<u64>>,
    appended: u64,
}

impl Reading {
    /// Starts counting at `now`.
    pub fn new(now: u64) -> Reading {
        Reading { appended: now, ..Reading::default() }
    }

    /// Counts the reader's event at `at` in `book`.
    pub fn event(&mut self, book: &Book, event: Event, at: Locator, now: u64, offset: i64) {
        match event {
            Event::Active => self.tally.active(now, offset),
            Event::Moved => {
                let chars = self.chars(book, at);
                self.tally.moved(now, offset, chars);
            }
            Event::Jumped => {
                let chars = self.chars(book, at);
                self.tally.jumped(chars);
            }
            Event::Paused => self.tally.paused(now, offset),
        }
    }

    /// `at` in characters (code points) from the book's start.
    fn chars(&mut self, book: &Book, at: Locator) -> u64 {
        if self.starts.is_empty() {
            let mut total = 0;
            self.starts = book
                .chapters
                .iter()
                .map(|c| {
                    c.blocks
                        .iter()
                        .map(|b| {
                            let start = total;
                            total += b.texts().iter().map(|t| t.text.chars().count() as u64 + 1).sum::<u64>().saturating_sub(1);
                            start
                        })
                        .collect()
                })
                .collect();
        }
        let Some(&start) = self.starts.get(at.chapter).and_then(|c| c.get(at.block)) else { return 0 };
        let block = &book.chapters[at.chapter].blocks[at.block];
        let text = block.texts().iter().map(|t| t.text.as_str()).collect::<Vec<_>>().join("\n");
        let mut byte = at.offset.min(text.len());
        while !text.is_char_boundary(byte) {
            byte -= 1;
        }
        start + text[..byte].chars().count() as u64
    }

    /// What to append now: everything when `leaving`, else only when a minute has passed since the last append.
    pub fn due(&mut self, now: u64, leaving: bool) -> Vec<Sum> {
        if !leaving && now < self.appended + APPEND_EVERY {
            return Vec::new();
        }
        self.appended = now;
        self.tally.drain()
    }
}

// endregion

// region Dates

/// The local day number (days since 1970-01-01) of `time` (milliseconds) in the zone `offset` seconds east of UTC.
pub fn day_number(time: u64, offset: i64) -> i64 {
    (time as i64 + offset * 1000).div_euclid(DAY_MS)
}

/// `yyyy-mm-dd` of the day number `day`.
pub fn date_string(day: i64) -> String {
    let (y, m, d) = crate::sync::civil(day);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The day number of `yyyy-mm-dd`.
pub fn parse_date(s: &str) -> Option<i64> {
    let mut parts = s.split('-');
    let (y, m, d): (i64, u32, u32) = (parts.next()?.parse().ok()?, parts.next()?.parse().ok()?, parts.next()?.parse().ok()?);
    if parts.next().is_some() || s.len() != 10 || !(1..=12).contains(&m) || d == 0 || d > days_in_month(y, m) {
        return None;
    }
    Some(days_from_civil(y, m, d))
}

/// The day number of the date (the proleptic Gregorian calendar).
pub fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (i64::from(m) + 9) % 12;
    let doy = (153 * mp + 2) / 5 + i64::from(d) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn days_in_month(y: i64, m: u32) -> u32 {
    match m {
        2 if (y % 4 == 0 && y % 100 != 0) || y % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

// endregion

#[cfg(test)]
mod tests;
