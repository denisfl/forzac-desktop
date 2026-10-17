//! The pages of the whole book for one page style (Android's background layout, the web reader's `BookPages`): the
//! menu's slider and "Page N of M", the foot's "N of M" and the page numbers of the lists. Counted in the background by
//! paginating every chapter as reading does; only each page's start is kept. The reading position stays a `Locator`:
//! book pages are derived from it.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};

use super::{PageStyle, Typesetter, paginate};
use crate::hyphen::Hyphenator;
use crate::model::{Book, Locator};

/// How long a count waits after the reader last moved before it typesets its next chapter: turns and scrolls draw
/// their frames without a typesetting thread beside them (on the Pi 5 one makes a turn's first frames late).
const QUIET: Duration = Duration::from_millis(500);

/// What a running count is told from the reader's thread: to stop, or that the reader moved (any thread).
pub struct CountControl {
    stop: AtomicBool,
    start: Instant,
    /// When the reader last moved, in ms after `start`; `u64::MAX` before the first movement.
    moved: AtomicU64,
}

impl Default for CountControl {
    fn default() -> Self {
        CountControl { stop: AtomicBool::new(false), start: Instant::now(), moved: AtomicU64::new(u64::MAX) }
    }
}

impl CountControl {
    /// Stops the count before its next chapter.
    pub fn stop(&self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    pub fn stopped(&self) -> bool {
        self.stop.load(Ordering::Relaxed)
    }

    /// The reader moved (a turn, a scroll, a frame of either): the count waits.
    pub fn moved(&self) {
        self.moved.store(self.start.elapsed().as_millis() as u64, Ordering::Relaxed);
    }

    /// Waits until the reader has not moved for `QUIET` (or the count is stopped).
    fn wait(&self) {
        loop {
            let moved = self.moved.load(Ordering::Relaxed);
            let now = self.start.elapsed().as_millis() as u64;
            let quiet = QUIET.as_millis() as u64;
            if self.stopped() || moved == u64::MAX || now >= moved.saturating_add(quiet) {
                return;
            }
            std::thread::sleep(Duration::from_millis((moved + quiet - now).min(50)));
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BookPages {
    style: PageStyle,
    /// Where each page starts, chapter by chapter.
    starts: Vec<Vec<Locator>>,
    /// The book page (from 0) of each chapter's first page.
    first: Vec<usize>,
    total: usize,
}

impl BookPages {
    /// Paginates every chapter of `book` for `style`; `None` when `control` is stopped before the last chapter is
    /// done. Before each chapter it waits while the reader moves (`CountControl::moved`). Blocks for as long as
    /// typesetting the whole book takes: run it on a worker.
    pub fn count(book: &Book, style: PageStyle, hyphenator: Option<Arc<Hyphenator>>, control: &CountControl) -> Option<Self> {
        let mut typesetter = Typesetter::new(style, hyphenator);
        let mut starts = Vec::with_capacity(book.chapters.len());
        for chapter in 0..book.chapters.len() {
            control.wait();
            if control.stopped() {
                return None;
            }
            let layout = paginate(book, chapter, &style, &mut typesetter);
            starts.push((0..layout.pages.len()).map(|i| layout.page_start(i)).collect::<Vec<_>>());
        }
        let mut first = Vec::with_capacity(starts.len());
        let mut total = 0;
        for pages in &starts {
            first.push(total);
            total += pages.len();
        }
        Some(BookPages { style, starts, first, total })
    }

    /// The style the pages were counted for: they are the reader's pages while it is the current one.
    pub fn style(&self) -> &PageStyle {
        &self.style
    }

    /// The book's pages.
    pub fn total(&self) -> usize {
        self.total
    }

    /// The book page (from 0) of each chapter's first page: the slider's ticks.
    pub fn chapter_starts(&self) -> &[usize] {
        &self.first
    }

    /// The book page (from 0) of page `page` of `chapter`.
    pub fn of_chapter_page(&self, chapter: usize, page: usize) -> usize {
        match self.first.get(chapter) {
            Some(&first) => (first + page).min(self.total.saturating_sub(1)),
            None => self.total.saturating_sub(1),
        }
    }

    /// The book page (from 0) holding `at`: the last page starting at or before it.
    pub fn page_of(&self, at: Locator) -> usize {
        let Some(last) = self.starts.len().checked_sub(1) else { return 0 };
        let chapter = at.chapter.min(last);
        let pages = &self.starts[chapter];
        let i = pages.partition_point(|s| *s <= at).saturating_sub(1);
        self.of_chapter_page(chapter, i)
    }

    /// Where book page `page` (from 0, clamped) starts.
    pub fn start(&self, page: usize) -> Locator {
        let page = page.min(self.total.saturating_sub(1));
        let chapter = self.first.partition_point(|&f| f <= page).saturating_sub(1);
        self.starts.get(chapter).and_then(|p| p.get(page - self.first[chapter])).copied().unwrap_or_default()
    }
}
