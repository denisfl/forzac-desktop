## Why

Past a hundred books the reader wants their own order: "To read in summer", "Work", "For the kids". Apple Books,
KOReader and Readest give collections plus automatic ones (review L1 in docs/ux-review.md); this is row 18 of
docs/roadmap.md, after the views of `library-metadata`.

## What Changes

- **Shelves are names on books.** "Add to shelf" in the book's "⋯" menu lists the shelves with a check on those the
  book is on — choosing one puts it on or takes it off — and "New shelf…", which asks for a name and puts the book on
  it. A shelf exists while a book is on it: taking the last book off removes it. A book can be on several shelves.
- **A fifth tab, "Shelves" ("Полки")**: the automatic shelves first — "Reading", "Not started", "Finished" (only when
  not empty) — then the reader's shelves A to Z, each a heading with its books; a book shows under every shelf it is on.
- Shelves are kept in the reading state by book id, so they survive renaming the file, re-layout, restarts and, like
  bookmarks, removing the book and putting it back from the Trash.

## Non-goals

- Renaming a shelf, ordering books by hand, a shelf sidebar, shelf covers; editing metadata (`metadata-editing`).
- Syncing shelves (`sync-format` will carry them).

## Capabilities

### Modified Capabilities
- `library`: the Shelves view, "Add to shelf" in the book's menu, the new shelf's question.
- `reading-state`: the shelves are kept.

## Impact

- `core/`: `shelf` lines in the state, `Entry` shelves from `scan`, `View::Shelves` in `arrange`, the strings; C ABI;
  tests.
- `gtk/`, `macos/`: the tab, the submenu with checks, the name's question.
- Docs: `docs/roadmap.md`.
