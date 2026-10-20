# Design

## Context

`library::scan` builds `Entry` values (title, authors, series from the folder cache; shelves from the state);
`library::book_info` builds "About the book" from a parsed `Book`. Per-book data lives in the state file by id
(`bookmark`, `quote`, `shelf` lines) and survives `forget`.

## Decisions

**1. Edits as an overlay by field.** `edit<TAB>id<TAB>field<TAB>value` lines, field one of `title`, `authors`,
`series`, `number`, `genres`; `state::Edits { title, authors, series: Option<Option<Series>>, genres }` where `None`
means "the book's own" and `Some(None)` for series means "no series". Only differing fields are kept, so a later fix in
the file still shows for the fields the reader did not touch.

**2. The form's text in the core.** `library::Fields` (five strings) with `fields(book, edits)` for the form and
`edits_from(book, fields)` for saving: trimming, empty-means-own, comma splitting and the number's parsing in one tested
place. GTK calls them directly; macOS through `forzac_edit_fields` / `forzac_save_edits` / `forzac_reset_edits`,
which take the open document and the state.

**3. Applied where the library reads.** `scan` applies a book's edits to its `Entry` (so views, search, covers'
labels and the generated cover's title follow), and `book_info` takes the edits. The reader window is untouched
(non-goal).

**4. The form.** GTK: a sheet like the new shelf's with five labelled entries and three buttons; macOS: an `NSAlert`
with a grid of labelled fields as its accessory and "Save", "Cancel", "From the book".
