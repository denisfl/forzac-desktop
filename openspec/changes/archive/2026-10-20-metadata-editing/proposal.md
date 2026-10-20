## Why

FB2 and EPUB files often carry poor metadata: a title in capitals, the author as "Clear, James" or in Latin letters,
no series. With the views and shelves the library sorts and groups by these fields, so the reader needs to fix them —
without Forzac rewriting files it did not make (`books-folder`). Row 19 of docs/roadmap.md; review L3.

## What Changes

- **"Edit…" in the book's menu** opens a form with Title, Authors (comma-separated), Series, Number in the series and
  Genres (comma-separated), filled with what the library shows; "Save", "Cancel" and "From the book", which drops the
  book's edits.
- Edits are kept by Forzac in the reading state by book id; the file is never changed. Only fields that differ from
  the book's own are kept; an empty title or authors means the book's own; an empty series removes the series.
- The library shows, sorts, groups and searches by the edited fields; "About the book" shows them.
- Edits survive restarts and, like shelves and bookmarks, removing the book and putting it back.

## Non-goals

- The reader window's title and the reading menu keep the book's own title for now.
- Editing the cover, the annotation, the language; writing metadata into files; editing several books at once.

## Capabilities

### Modified Capabilities
- `library`: the "Edit…" item and its form; the edited fields in the views, search and "About the book".
- `reading-state`: edits are kept.

## Impact

- `core/`: `edit` lines in the state, `library::Fields` (the form's text) with `fields` / `edits_from`, edits applied in
  `scan` and in `book_info`; strings; C ABI; tests.
- `gtk/`: the form as a sheet; `macos/`: the form as an alert with fields.
- Docs: `docs/roadmap.md`.
