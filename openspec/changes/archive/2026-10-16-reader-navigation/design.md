## Context

The GTK shell turns pages, jumps between chapters and remembers the position (`reading-state`). The core lays out
chapters lazily (the neighbours in the background) and produces page contents with active areas for note markers and
links (`typesetting-complete`); the book has contents (`TocEntry` with Locators), notes and resolved link targets. The
page foot is drawn by the core: "4 / 22". Reader shortcuts are on the window with global scope.

## Goals / Non-Goals

**Goals:** contents, notes, links with a way back, progress on the page, mouse and wheel, in the GTK shell.

**Non-Goals:** the macOS shell (it catches up in `macos-app`); book-wide page numbers; bookmarks (`bookmarks`); search
(`book-search`); a forward jump list.

## Decisions

### Positions in percent, not page numbers
Page numbers for the whole book need every chapter laid out for the current window and text size, which the lazy
layout avoids on purpose (a big book takes most of a second on the Pi). The contents and the foot show the position in
the book as a share of its text instead: cheap, stable across window sizes, and the same measure as the recent-books
list.

### Core
- `ChapterLayout` keeps, from `paginate`, the chapter title and the text length before the chapter, in each of its
  blocks, and in the whole book, so a page's position is a prefix sum. `render` draws the running head (not on page 0)
  and the foot "n / N · p%"; text that does not fit is shortened with "…".
- `ChapterLayout::height(page)`: how tall the page's content is, to size the note panel.
- `notes::note_book(book, id) -> Option<Book>`: a one-chapter book of the note's title (as a heading) and blocks, sharing
  the images, so the note is typeset by the usual `paginate` at the panel's width on a very tall page and drawn with the
  same code as a page.
- `state::progress` stays the measure for one position; the contents use it per entry (one pass over the book for all
  entries).

### GTK
- **Drawing shared**: building a render node from `PageContent` (fonts, inks, rules, textures) moves out of `PageView`
  into a function used by the page and by a small `NoteView` widget.
- **Areas per page** are cached with the page node; hit-testing a click and `Tab` focus use them. The focus highlight is
  drawn over the cached node (a rounded rectangle in the link ink at low opacity), so focus changes only redraw.
- **Panels** are children of the window's overlay: the contents on the right (the theme's colours through a CSS
  provider updated with the theme), the note at the bottom. A panel takes the focus while open.
- **The contents are a `GtkListView`** (only the rows on screen exist), built once per window in the first pause in
  reading (a second without a move, so it never delays a turn) and shown once invisibly then, so the interface font
  and styles are loaded before `o`. Measured on the Pi: a `GtkListBox` of all entries took 0.8–0.9 s to appear; the
  list view 60–130 ms; prepared, 30–49 ms, and 6 ms when opened again.
- **Tab stops** are runs of areas with the same target, so a link broken across lines is one stop; the core merges
  a link's words on a line into one area.
- **Shortcut scope**: the reader shortcuts move from the window (global scope) to the page view (local scope), so they
  act only while the page has the focus; panels have their own keys. `q`, `f`, `?` stay window-wide.
- **Jumps** (`go_to(Locator)`) push the current position on a back stack (50 places); `Ctrl+O` pops. A jump into
  another chapter uses that chapter's layout (from the neighbours if ready) and shows its page without the slide.
- **External links** open through `GtkUriLauncher`.
- **Wheel**: discrete wheel steps turn one page each; smooth (touchpad) scrolling accumulates 60 px for one turn and
  turns at most once until the gesture ends.

## Risks / Trade-offs

- Percentages are coarser than page numbers in a long book → the foot also shows the page within the chapter.
- A note longer than half the window scrolls in its panel instead of paging → notes are short in practice.
