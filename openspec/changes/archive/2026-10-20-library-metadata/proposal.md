## Why

The library shows one shelf, the last opened first. Past a few dozen books — and `folder-import` brings hundreds — a
reader looks for a book by its title, its author or its series, and the books know these: FB2 has `sequence`, `genre`
and `date`, EPUB has Calibre's series and `dc:subject`, FB3 has its sequence, classification and date. This is the first
part of the Calibre-like library (docs/roadmap.md, 17; review L1–L2 in docs/ux-review.md).

## What Changes

- **Metadata:** the core reads a book's series and number in it, genres, year and language besides title, authors and
  annotation, from FB2, FB3 and EPUB 2/3. FB2 genre codes are shown by name in English and Russian.
- **Views:** tabs under the library's head — "Recent · Title · Author · Series" (Недавние · Название · Автор · Серия).
  Recent is today's shelf with "Continue reading"; Title is one shelf A to Z; Author groups the books under each
  author's name, authors A to Z by surname; Series groups them under each series, books in their order, books outside a
  series last. `Tab` / `Shift+Tab` or a click switch views; the view is remembered.
- **Under a cover:** the series and number ("Полдень, XXII век · 2") when the view is not Series.
- **The book's "⋯" menu** in place of the "×": under the pointer or on the selected book, and on a right click — "About
  the book" and "Remove from library"; the menu is where `library-shelves` and `metadata-editing` add their items.
- **About the book:** a sheet over the library with the cover, title, authors, series, genres, year, language and the
  annotation, and "Read"; `i` opens it for the selected book, `Esc` closes it.
- **Search** also matches the series' name.

## Non-goals

- Shelves and editing (`library-shelves`, `metadata-editing`); a genre view or genre filter; publishers, ISBN, rating.
- A list or table view like Calibre's: the cover grid stays.

## Capabilities

### New Capabilities
- `book-metadata`: what is read from each format.

### Modified Capabilities
- `library`: the views and groups, the series under covers, the "⋯" menu instead of "×", the book's sheet, search over
  series, the keys.

## Impact

- `core/`: the model's series, number, genres, year; the three parsers; FB2 genre names; the folder cache's `file`
  lines get series and number (older lines are read again once); `library::arrange` (views and groups) and
  `library::step` (moving the selection across groups); the book's information; C ABI; tests.
- `gtk/`, `macos/`: the tabs, group headings, selection across groups, the "⋯" menu, the sheet, the keys.
- Docs: `docs/roadmap.md`.
