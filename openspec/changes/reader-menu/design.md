## Context

The reference is the web reader (`~/dev/forzac/site`: `public/read/index.html` `#bar-top`/`#bar-bottom`,
`public/read/reader.css` `.bar*`, `.slider*`, `.ticks`, `.aa`; `src/read/js/app.ts` `setMenu`, `showMenuPlace`,
`ticks`, `tapZone`; `src/read/js/pages.ts` `BookPages`) and Android's `ReaderMenu.kt`. Both count the book's pages in
the background and show nothing page-based until the count is done.

Today the desktop core paginates one chapter at a time (`layout/page.rs` `paginate`); GTK keeps the current chapter and
its neighbours (typeset on a `gio::spawn_blocking` worker), the Mac keeps five layouts in the `Document` and typesets on
the main thread (`ffi.rs` `Document::layout`). The foot is drawn by the core into each page (`ChapterLayout::render`).
Lists (contents, bookmarks, highlights, search) show percent from `state::progress_of`.

## Goals / Non-Goals

**Goals:** the web menu's look (sizes, colours from the theme palette both shells already have, the Golos UI face of
the panels) and behaviour in both shells; whole-book page numbers derived from the `Locator`, never stored as the
position; no frame missed because of the count.

**Non-Goals:** see the proposal. No cache of counts on disk; no new setting.

## Decisions

### Core: `BookPages` (`core/src/layout/pages.rs`)

```rust
pub struct BookPages { style: PageStyle, starts: Vec<Vec<Locator>>, first: Vec<usize>, total: usize }
impl BookPages {
    pub fn count(book: &Book, style: PageStyle, hyphenator: Option<Arc<Hyphenator>>, stop: &AtomicBool) -> Option<Self>;
    pub fn style(&self) -> &PageStyle;      // what it was counted for
    pub fn total(&self) -> usize;
    pub fn chapter_starts(&self) -> &[usize]; // book page of each chapter's first page (the ticks)
    pub fn page_of(&self, at: Locator) -> usize;   // book page (from 0) holding `at`
    pub fn start(&self, page: usize) -> Locator;   // where book page `page` starts
}
```

`count` runs `paginate` for every chapter with its own `Typesetter` (same style, same hyphenator: the same pages as
reading, by construction) and keeps only each page's start `Locator` (`ChapterLayout::page_start`): about 24 bytes a
page. `stop` is checked between chapters; a stopped count returns `None`. The shaped-word cache grows over the whole book
and is dropped with the typesetter at the end. Alternative considered: reuse the reading layouts as they appear — no
total until every chapter is read; rejected.

`PageStyle` holds everything that changes typesetting (size, family, spacing, alignment, margins, window size), so a
count is valid exactly while `pages.style() == current style`; theme changes do not touch it.

`ChapterLayout::render_numbered(i, style, typesetter, pages: Option<&BookPages>)` draws the foot as "N of M"
(`tr("{n} of {m}")`, "N из M") when `pages` matches the layout's style; `render` stays for callers without a count.

Also in the core: `model::Book::section_at(at) -> String` (the contents entry at or before `at`, else the book's
title: the web's `chapterTitle`), and `model::Book::minutes_left(at) -> usize` (characters — not bytes — from `at` to the
chapter's end, `/ 1000`, rounded).

### Running the count

- **GTK** (`page_view.rs`): `State` gains `pages: Option<Arc<BookPages>>` and `counting: Option<Arc<AtomicBool>>`. After
  the first frame, and 300 ms after the last style change (a `glib::timeout` restarted by each change, so a live resize
  does not start a count per pixel), `count_pages()` sets the old job's stop flag and runs a new one on
  `gio::spawn_blocking`. On arrival the result is kept if its style is still current; the page nodes are dropped
  (their foot changes) when no turn or scroll runs, else after it (a 50 ms retry), so no frame of an animation renders
  pages again. The menu and the lists read `view.book_pages()`.
- **macOS** (C ABI): a job object captures the `Arc<Book>`, the style and the hyphenator on the main thread, runs on a
  background `DispatchQueue` and is installed back on the main thread:

  | Function | Thread | What |
  |---|---|---|
  | `forzac_count_new(doc, width, height, settings) -> *mut Count` | main | a job for the current style |
  | `forzac_count_run(count)` | any | counts (blocking) |
  | `forzac_count_stop(count)` | any | asks a running count to stop |
  | `forzac_count_install(doc, count) -> bool` | main | moves the result into the document if its style is current |
  | `forzac_count_free(count)` | main | |
  | `forzac_book_pages(doc) -> u32` | main | the book's pages, 0 when not counted for the current style |
  | `forzac_book_page_of(doc, at) -> u32`, `forzac_book_page_start(doc, page) -> ForzacLocator` | main | |
  | `forzac_chapter_starts(doc, *count) -> *const u32` | main | the ticks |
  | `forzac_section(doc, at) -> *const c_char`, `forzac_minutes_left(doc, at) -> u32` | main | the labels |

  `forzac_page` draws the foot with the installed count. `forzac_paginate` drops a count of another style. On install
  the Mac drops its page bitmaps and shows the current page again (Core Animation keeps animating in the window server).

The count runs on another core than drawing: the Pi 5 has four. If the Pi measurement shows missed frames, the
fallback is to let the job wait between chapters while a turn or scroll runs (the web's `busy()`); measured first.

### The menu in GTK (`gtk/src/menu.rs`)

Two overlay children of the window's `gtk::Overlay`: the top bar (`halign` fill, `valign` start) and the bottom bar
(`valign` end), `forzac-panel` boxes so they take the panels' theme CSS (paper, ink, muted, line), Golos Text. The
icons are `icons::Icon` with Lucide's chevron-left and list. The slider is a small custom widget (`gtk::DrawingArea` and
a `GestureDrag`) drawing the track, ticks, progress and thumb exactly as `ReaderMenu.kt`'s `ProgressSlider` (8 px
radius inset). Fade and slide by CSS: `.reader-bar { opacity: 0; transform: translateY(∓50%) }` and `.reader-bar.open`
with `transition: opacity, transform` 200 ms in / 150 ms out; the class is added a frame after `set_visible(true)` and
the bars are hidden 150 ms after the class goes, so a closed menu is not picked or drawn. The page view does not redraw
for the bars (its node is cached).

Keys: the window's shortcut controller gets the menu first through a capture-phase key controller on the page view:
while the menu is open `Left`/`h`/`Right`/`l` move the scrub, `Return`/`KP_Enter` jumps (or closes when unmoved),
`Escape` closes, and anything else closes the menu and proceeds to the usual shortcut. `Return` with no focus calls
`menu.open()` instead of `turn(1)`.

Clicks: `PageView::click` gets a `on_middle` callback: a click in the middle third, after markers, links, the corner,
highlights and the selection had their turn, toggles the menu; any click while the menu is open closes it and does
nothing else (the window's capture gesture, as for panels). A double click thus opens the menu for the interval and the
second click closes it and selects the word (the web reader does the same order).

Back: the window closes and `recent_window` is shown if no other one is (a two-line change next to the `library`
session's work: the `library` change replaces `recent_window`).

### The menu on macOS (`macos/Menu.swift`)

`ReaderMenu: NSView` holding two layer-backed bar views over the `PageView` in the reader's container; labels are
`NSTextField`s in the Golos face the panels use, buttons draw the same Lucide paths (`Panels.swift` already draws icons
from paths), the slider is an `NSView` with `CALayer`s for the track, ticks, progress and thumb. The fade and slide are
`NSAnimationContext` animations of `alphaValue` and the frame's origin (200 ms in, 150 ms out). `PageView.keyDown` asks
the menu first while it is open; `Return` without focus opens it; `mouseUp` in the middle third toggles it.

### Labels and lists

New strings (English → Russian, the Android/web words): "Back to library" → "К библиотеке", "Page {n} of {m}" →
"Страница {n} из {m}", "{n} of {m}" → "{n} из {m}", "{n} min left in chapter" → "{n} мин до конца главы", "Less than a
minute left in chapter" → "Меньше минуты до конца главы", "p. {n}" → "стр. {n}", "Text settings" → "Настройки текста",
"Position in book" → "Место в книге". Contents and bookmarks: the number column shows the bare page number once
counted (web `.toc-list .n`), else the percent as now. Highlights and search: "section · p. N", else "section · N%".
The help: `Enter` is listed as "menu" instead of a turn.

## Risks / Trade-offs

- **CPU contention on the Pi** while counting a long book (seconds of one core). Mitigation above; measured with
  `--turns` right after opening, when the count runs.
- **Recounting after every resize**: the foot falls back to chapter pages for the count's duration, as Android does
  after a rotation. Accepted.
- **`Enter` changes meaning** (was "next page"): the maintainer's decision for this change; the help says so.
- **Memory**: a count's typesetter holds the shaped words of the whole book until it ends (a few MB); pages themselves
  are ~24 bytes each.
- **The double click in the middle** flashes the menu for one click. Accepted, as the web reader.
