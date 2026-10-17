## Context

Android keeps a bookmark as the Locator of the page start with the chapter title and the page's first 160 characters
(`ReaderViewModel.kt` `toggleBookmark`, `Library.kt` `Bookmark`); a page is marked when a bookmark lies between its
start and the next page's start; a tap in the top right corner sets one on an unmarked page and removes all of a marked
one. The mark is a bordeaux ribbon with a swallowtail end in the right margin (`ReaderScreen.kt` `Ribbon`): it drops
with a spring on adding, goes up on removing, and swings like cloth while pages move. The list is a tab of the contents
(`TocScreen.kt` `BookmarkList`): chapter, page number, two lines of text; empty, a hint.

On the desktop the reading state is one text file (`core/src/state.rs`), the contents panel is a side panel in each
shell (`gtk/src/panels.rs` `Contents`, `macos/Panels.swift` `ContentsPanel`), and lists show positions in percent
(android-parity R1).

## Goals / Non-Goals

**Goals:** the Android behaviour from the keyboard (`b`, `B`, `d`), the ribbon, the list as a tab, persistence by
Locator.

**Non-Goals:** the ribbon's cloth swing while pages move (a desktop turn is a 200 ms slide started by a key; the swing
would draw frames after it ends and is the part of the Android ribbon least worth its cost); notes on bookmarks; a
bookmark per line; exporting bookmarks.

## Decisions

**Storage: one line per bookmark in the state file (core).** `bookmark<TAB>book-id<TAB>chapter<TAB>block<TAB>offset`.
`Store` reads them with the books and settings and writes them with the same re-read-and-replace, so other processes'
bookmarks survive. Section and text are not stored: the book is open whenever they are shown, and a book with other
content has another id. An older build drops the lines at its next write (nothing is published yet).
`Store::bookmarks(book)` and `Store::set_bookmarks(book, &[Locator])` replace a book's set in one write.

**Toggle and list entries in the core (`core/src/bookmarks.rs`).** `toggle(&mut marks, from, to)` adds `from` when no
bookmark lies in `[from, to)`, else removes those that do, and keeps the list sorted; `marked(marks, from, to)`;
`entries(book, marks)` gives, in one pass, each bookmark's section (the contents entry at or before it, as search
does), its share of the book (`state::progress_of`) and a snippet: the text from it on, up to 160 characters across
blocks, note markers, soft hyphens and line breaks dropped, white space collapsed. `ChapterLayout::page_end(i)` is the
next page's start or the next chapter's start, so a shell asks for a page's range in one call.

**The shell owns the range.** "Slide": `[page_start(i), page_end(i))`. "Scroll": from the top line (the line at the top
margin, `flow_locator`) to the first line at or below the window's bottom, in the next chapter when the window reaches
into it (`Locator { chapter + 1, 0, 0 }` past the book's last chapter). The same range decides whether the ribbon shows,
so what `b` removes is what the ribbon promises.

**C ABI.** The document holds its bookmarks, as it holds the book: `forzac_state_load_bookmarks(state, doc)`,
`forzac_state_save_bookmarks(state, doc)`, `forzac_bookmark_toggle(doc, from, to)` (true when set),
`forzac_bookmark_remove(doc, at)`, `forzac_bookmarked(doc, from, to)`, `forzac_bookmark_places(doc, &count)` (the
Locators, for "Scroll"), `forzac_bookmarks(doc, &count)` (entries with section, snippet and position; valid until the
next bookmark call) and `forzac_page_end(doc, page)`. The GTK shell uses the same Rust functions directly and keeps the
list in its view's state; both shells save after every change (measurement runs keep bookmarks in memory only).

**The ribbon is drawn by the shells, not into the page.** Its geometry is Android's: 16 × 44 px below the window's top
edge (12 px more tucked above it), a swallowtail 9 px deep (9/16 of the width), bordeaux by the theme's lightness. Its
centre is `min(margin / 2, 40)` right of the text column's right edge: Android centres it in the margin, but a desktop
margin can be hundreds of pixels wide and the ribbon belongs to the text block. GTK draws a filled `gsk::Path` under a
shadow node over the page node (cached page nodes stay as they are), translated with each page during a slide (`Turn`
remembers whether the page it slides away was marked). macOS gives each of the two page layers a `CAShapeLayer` with the
path and a shadow, so the ribbon slides with its page in the window server; in "Scroll" the current layer's ribbon hangs
at the top of the window.

**Animation.** The drop is Android's spring (damping ratio 0.5, stiffness 280 s⁻², from 56 px above): on macOS a
`CASpringAnimation` with mass 1, stiffness 280, damping 16.7; in GTK the same closed-form damped oscillation on the
frame clock until it is within half a pixel (about 0.55 s). The rise is a 200 ms ease-in to 56 px above. A turn, a jump
or a re-layout ends a running animation. In "Scroll" the ribbon appears and goes with the marked screen without
animation, as on Android.

**Keys and the panel.** `b` toggles on the page (nothing on the end page). The contents panel gets two tabs in its
header in place of the title — Golos 17, the selected one semibold in the ink, the other medium in the secondary ink,
12 px side padding as on Android — then the close button. The filter field shows on "Contents" only. The bookmarks
list rows follow Android's: section (PT Serif 16, one line) with the percent on the right (13 px secondary, 44 px), the
snippet below (Golos 13 secondary, two lines), 12 px above and below. `←`/`→`, `h`/`l` and `Tab`/`Shift+Tab` switch
tabs on a list; `B` on the page opens the bookmarks tab (or closes the panel when it shows it). `d` or `Delete` removes
the selected bookmark, keeps the selection's index and redraws the page without animation (it is under the panel).
Going to a bookmark is a jump like a contents entry (`Ctrl+O`, the pill).

**The corner click.** In both modes a click in the top 56 px (the top margin) from 44 px left of the column's right
edge to the window's edge toggles, after note markers and links and before the turning thirds. It is the Android tap
for the mouse.

**Mac menu.** Go → Bookmark `⌘D` (Safari's bookmark key), View → Bookmarks.

## Risks / Trade-offs

- [The "Scroll" range needs `flow_locator` twice per frame while the column moves] → It walks the lines of one chapter
  (a few thousand at most): microseconds; measured with `--scrolls` on the Pi.
- [Two processes toggling bookmarks of one book at once] → Each write re-reads the file and replaces only that book's
  bookmarks with its own set: the last write wins for that book, as for its position.
- [A bookmark saved before a file changes] → The changed file is another book (its id), so its bookmarks start empty;
  the old ones stay with the old id, as positions do.
