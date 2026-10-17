## Context

The recent-books window (`gtk/src/main.rs` `recent_window`, `macos/main.swift` `RecentWindow`) is a plain list of up
to 20 books from the reading state (`core/src/state.rs` `Store::recent`). The web reader's library
(`~/dev/forzac/site/public/read/index.html` `#library`, `reader.css` "Библиотека" and "Полка", `src/read/js/app.ts`
`showLibrary`) is a centred column (at most 1120 px) with a head (brand, "Open a book" pill), a note line, a grid of
2 : 3 covers with title and author, a "×" to remove, a dashed drop overlay and an empty state with an illustrated
page. The Android library (`LibraryScreen.kt`, `Covers.kt`) adds the "Continue reading" card, the share read under
covers, generated covers, search and a question before deleting. The core has the cover image of a parsed book
(`Book::cover`, `forzac_cover`), but no image codec; the shells have one (gdk-pixbuf / GDK textures, ImageIO).
A parallel change, `reader-menu`, adds "← Back to library" to the book window: it closes the book window, and what
follows the last book window shows the library.

## Goals / Non-Goals

**Goals:** the web library's look and behaviour in both shells, plus the Android card, progress, generated covers,
search and removal question; keyboard first; 50 books in at most 200 ms on the Pi 5; nothing drawn while idle; a small
state file.

**Non-Goals:** importing, collections, sorting choices, the card's chapter title and time left, the contents panel's
cover.

## Decisions

**The web's layout, the Android's additions.** The window is the web library: a column at most 1120 px wide with
side padding of 4 % of the width (20–48 px); the head (brand: the icon 36 px and "Forzac" in Literata Bold 22; on the
right a 44 px circle with Lucide's search icon, as Android's header, then the ink pill "Open a book", Golos Medium
15); the note line (Golos 14, secondary colour); the Android card and a 1 px rule; the shelf (`auto-fill`, columns
`clamp(130, 13 % of the width, 176)` px, gaps 36 px down and `clamp(20, 2.4 %, 32)` px across); under each cover the
title (Golos Medium 14), the authors (Golos 13, secondary) and the share read (Golos 12, secondary, Android's
`progressLabel`). The share read was not on the web shelf; the task asks for it and Android shows it. The web shows
the remove "×" on hover; the desktop does the same and adds `Delete`/`d`.

**The card.** Android's `continueBook`: the first book by last opened whose progress is below 0.999. Cover 92 × 138,
"CONTINUE READING" (Golos Medium 12, letter spacing, secondary), the title (serif semibold 21), the authors (Golos 14,
secondary), a 2 px bar (line colour, ink up to the share read) and the share read. Android's chapter title and time
left need the book's chapters and length, which the state does not keep: left out rather than growing the state or
parsing every book on opening the library.

**Removing asks; it does not undo.** The web removes at once, Android asks. An undo needs a timer and a toast that
lives on after a keypress; a question is one more key and cannot be missed. The question is Android's bottom sheet
(`DeleteSheet`) in the window: "Remove “title” from the library?", "The book file stays where it is.", "Cancel"
(outlined) and "Remove" (ink). `Enter` removes, `Esc` cancels. Removing calls `Store::forget(id)`: the `book` line goes;
`bookmark` and `quote` lines stay keyed by the book's id, so reopening the file brings them back (only the position
is lost, as the entry holds it). The cached thumbnail is deleted.

**State.** `Store::books()` lists every entry whose file exists, last opened first (`recent(n)` stays for Open
Recent). `forget(id)` drops the entry in one write. `ForzacRecent` gains the book's `id` (the cover cache key and
the generated cover's seed); `forzac_state_forget(state, id)`. The library reads the state once when it is shown.

**Cover cache: JPEG thumbnails beside, not in, the state.** Decoding 50 full-size covers (often 1–2 MB JPEGs) costs
tens of ms each on the Pi; the state file must stay small. So each shell keeps `<cache>/covers/<id>.jpg`, at most
352 × 528 px (176 pt at 2×), JPEG quality 85: GTK `$XDG_CACHE_HOME/forzac/covers` through gdk-pixbuf (scaled while
loading, saved as JPEG), macOS `~/Library/Caches/Forzac/covers` through ImageIO
(`CGImageSourceCreateThumbnailAtIndex`, `CGImageDestination`). An empty file means "this book has no cover", so it is
not looked for again. A thumbnail is made on a background thread when a book window opens (the reader's book is
parsed already), written to a temporary file and renamed. While the library is shown, books without a cache file are
parsed one at a time on one background thread (`forzac_core::open` / `forzac_open`), oldest last, to fill the cache
for books read before this change; their covers appear when ready. The library loads thumbnails on a background
thread too (GDK textures and `CGImage`s are immutable and can cross threads), so the first frame never waits for
decoding: a cover is drawn in the line colour until its picture arrives, as the web's `.cover` background does.

**Generated covers in the core's numbers, drawn by the shells.** `core/src/library.rs` has Android's choice of
material and colours from the id (`hasMarbleCover`: `hashCode(id).abs % 5 < 2`; cloth from six colours by
`hashCode(id).abs % 6`; Java's `String.hashCode` over UTF-16 so a book looks the same in every run), the marbled
paper (`Marble.render` with `Mulberry32`, its three palettes, seeded by `hashCode(title)`) as RGBA pixels at half the
cover's size (88 × 132), and the title's size factor by length (`byLength`). The shells draw the cloth or the marble
(rendered on a background thread; solid indigo until then, as Android), the spine (9 %, the cloth darkened to 78 %, or
indigo), the paper label (from 20 % of the height, insets 8 %, ochre frame 0.8 % + 0.5 px) with the title in PT Serif
Bold (base 11.5 % of the width × the length factor, shrunk to fit the longest word down to 0.6, at most 4 lines, 5
when longer than 50) and the authors in capitals (Golos 46 % of the base). Android draws the title in its system
serif (Noto Serif), which Forzac does not bundle; PT Serif is the serif the panels already use.

**The cover's shape** is Android's `BookShape`: a 2 px corner at the spine, 6 % of the width at the edge; the web's
shadow (`0 1px 2px` 12 %, `0 6px 18px` 10 %). A selected cover has a 2 px ring in the ink, 3 px outside it.

**GTK: two small widgets, not `FlowBox`.** `FlowBox` cannot give the web's columns (min/max width with the remainder
spread over them) and does not tell the column count that `j`/`k` need. `Shelf` is a widget with a grid layout of
its own (height for width: columns from the width, rows from the items' heights) and `Cover` draws one cover in its
snapshot (`append_outset_shadow`, a rounded clip with per-corner radii, the texture scaled to cover, or the generated
cover with Pango layouts). An item is a vertical box: cover, title row (title and the "×", shown on hover), authors,
share. The window is a `ScrolledWindow` over the column, with an overlay for the drop frame and the question sheet.
Keys go through one `EventControllerKey` on the window (keyvals, so a Russian layout works through GTK's Latin
fallback, as the reader's shortcuts do); the search field takes text keys while it has the focus. Drop: a
`DropTarget` for `gdk::FileList`. File dialog: `gtk::FileDialog` with a "Books" filter (FB2, FB2.ZIP, FB3, EPUB, ZIP).

**macOS: one flipped document view with manual layout.** An `NSScrollView` whose document view lays out the same
column in `layout()` (heights from the width); `CoverView` draws in `draw(_:)` with Core Graphics and Core Text. Keys
through a local key monitor on the window, by key code (`shortcutKey`), as the reader. Drop: the content view
registers `.fileURL`. File dialog: the existing `openDocument`.

**The empty state** is the web's: on the left the heading (Literata Bold, 34–52 px by width), the lead (Literata 18–21),
the big pill, the hint, the note line under a rule; on the right the illustration — a page (sheet colour, radius 6,
shadow `0 18px 48px` 12 %) over a second turned 3.2° and faded to 70 %, with the running head, the chapter title, three
justified hyphenated paragraphs in Literata and the folio, the font size 3.55 % of the page's width. The heading and
lead are the web's adapted to a desktop app: the web's "Read books right in your browser" becomes "Read books right on
your computer" ("Читайте книги прямо на компьютере"), and its lead ("with the same typesetting… as in the app") would
be untrue here, so it says what the desktop does differently: the file stays where it is. The sample page's texts are
the web's (Pushkin in Russian, Austen in English) and live in the core (`library::sample_page()`, `forzac_sample_page`),
like the help texts: they are content per language, not translations of each other. Narrower than 860 px the page
goes under the text, as the web's media query.

**Search** is Android's `librarySearchRank` ported to the core (`library::rank(query, title, authors)`: words by
Unicode letters and digits after lower-casing and "ё" → "е"; per query word: a title word's prefix 0, its transliterated
skeleton 1, an author word's prefix 2, its skeleton 3, else no match; the sum sorts). The field replaces the brand
(PT Serif 26, Android's `SearchHeader`), with "×" closing it. While searching the card is hidden and the shelf holds
the matches.

**Window and lifetime.** One library window at a time (a weak reference in GTK, `recent` in the Mac delegate): a
message for a dropped non-book does not open a second one. Opening a book from the library closes the library, as the
recent books did. Default size 1040 × 760 (85 % of the monitor at most); the column centres in wider windows.

**Strings.** In the catalogue with the web's words where the web has them ("Open a book" / "Открыть книгу", "or drop a
file into this window", "Release the file to open it", "Remove from library" / "Убрать из библиотеки", the note
line) and Android's otherwise ("Continue reading" / "Продолжить", "Title or author" / "Название или автор", "No
results" / "Ничего не найдено", "Cancel" / "Отмена", "Finished" / "Прочитано", "Search" / "Поиск"). The question is
Android's `delete_book_title` with "Remove … from the library" for "Delete": the file is not deleted.

**Test flags.** GTK: without a book, `--measure` prints the time from the start to the library's first presented
frame and to the last cover loaded, then quits; `--png out.png` saves the library window (as rendered) and quits;
`XDG_DATA_HOME` and `XDG_CACHE_HOME` choose the state and the cache. macOS: `--library` with `--png out.png` saves
the library window, `--keys` sends keys to it first, `--state PATH` reads and writes another state file (also without
`--library`, for by-hand runs).

**Merging with `reader-menu`.** That branch closes the book window to go back; this one keeps the hooks it relies on
(`connect_window_removed` in GTK, `Reader.onClose` on the Mac) and renames `recent_window`/`showRecent` to
`library_window`/`showLibrary`; a call to the old names is the only expected conflict.

## Risks / Trade-offs

- [Parsing old books for their covers takes CPU while the library is open] → one book at a time on one thread,
  nothing on the main thread, done once per book (the empty marker file).
- [The JPEG cache and the state disagree after a removal in another process] → a stale thumbnail is only a file in the
  cache; it is used again if the book comes back.
- [The Literata variable font's bold through fontconfig or Core Text] → checked on both machines in the by-hand pass.
- [Generated covers differ from Android's in the serif] → PT Serif is the panels' serif; noted in
  `docs/android-parity.md`.
