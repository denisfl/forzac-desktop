# Design

## Context

`model::Book` has title, authors, language, annotation, cover. The library lists `state::Entry` values from
`library::scan`: remembered books plus the Books folder's files, whose title and authors are cached in the state's
`file` lines (name, size, mtime, id, title, authors). Both shells lay out one card and one shelf grid and move the
selection with their own `step` over (card, columns). The "×" under the pointer asks to remove a book.

## Decisions

**1. Series in the cache, the rest on demand.** Sorting and grouping need the series and its number for every book, so
the `file` line gets two more fields (series, number) and `Entry` carries them (filled from the cache by `scan`; not
written in `book` lines). Genres, year and annotation are only on the sheet, read from the file when it opens. A
7-field `file` line from 0.11 is dropped when read, so the next scan parses those files again once.
*Alternative:* everything in the cache — a bigger state file for data shown one book at a time.

**2. Views and groups in the core.** `library::View { Recent, Title, Author, Series }` and
`library::arrange(books, view) -> Vec<Group { heading: Option<String>, books: Vec<usize> }>`, with the comparison key
(lowercase, ё→е, leading punctuation dropped) and the surname rule in one place, tested once for both shells. The view
is the setting `library.view`.

**3. Moving the selection in the core.** `library::step(rows, at, dx, dy) -> usize` over the laid-out groups: the
shells pass, for every group, its number of books and columns (the card as a group of one); `j` from the last row of a
group goes to the same column of the next group's first row (or its last book), `k` back; `h`/`l` move within a row
and wrap across groups as reading order. One tested function instead of two hand-written ones.

**4. The "⋯" menu is native.** GTK `gtk::PopoverMenu` from a `gio::Menu` with two actions; macOS `NSMenu` popped up at
the button (and as the item's context menu). Native menus get the system's keyboard handling and accessibility; later
changes add items to the same menu.

**5. The sheet reuses the question's backdrop.** Both shells already dim the library and show a sheet for "Remove…?";
the book's sheet uses the same backdrop and colours, with the cover from the thumbnail cache, the text from
`library::BookInfo` (title, authors, series, number, genres, year, language name, annotation) read in the background
(GTK `gio::spawn_blocking`, macOS a background queue; C ABI `forzac_book_info`).

**6. FB2 genre names.** A table of the common FB2 genre codes (the FB2 2.1 list's frequent ones: prose, sci-fi,
detective, adventure, children's, poetry, science, computers, reference, religion, humour, home…) with English and
Russian names in `i18n`; unknown codes are left out rather than shown raw.

## Risks / Trade-offs

- The first library show after the update re-parses every book in the folder to fill the series (seconds for hundreds
  of books on the Pi 5, once).
- The surname is the last word that is not an initial; wrong for some names ("Le Guin", Chinese order); acceptable until
  `metadata-editing`.
