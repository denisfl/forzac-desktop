# Design

## Context

The state file keeps per-book lists by id (`bookmark`, `quote` lines). `library::scan` gives `Entry` values with
title, authors, series, progress and `opened`; `library::arrange` makes the views' groups; both shells build the "⋯"
menu (`gio::Menu` + actions in GTK, `NSMenu` on macOS).

## Decisions

**1. A shelf is a name on a book.** `shelf<TAB>id<TAB>name` lines, one per book and shelf; `Store::shelves_of(id)`,
`Store::shelf_names()` (A to Z, distinct without case), `Store::set_shelf(id, name, on)`. No list of shelves of its
own: a shelf with no book is gone, which removes "delete" and "empty shelf" from the interface. Names are matched
without case, the first spelling kept. *Alternative:* shelves as objects with ids (rename, order, covers) — more UI and
sync surface than asked for.

**2. Shelves in `Entry`.** `scan` fills `Entry::shelves` from the store (not written in `book` lines), so `arrange`
works on entries as for the other views. `View::Shelves` arranges the automatic shelves (by `opened` and `progress`
against `FINISHED`) then the reader's; a book appears once per shelf it is on, so the shells' items may repeat a book —
selection and `step` work on items, not books.

**3. Menus.** GTK: the submenu's shelves are boolean-state actions made each time the menu opens (`shelf.0`, `shelf.1`,
… in their own action group), "New shelf…" an action with the book; macOS: `NSMenuItem.state = .on` for membership.
The name is asked in a sheet like the remove question with an entry (GTK), an `NSAlert` with a text field (macOS).

## Risks / Trade-offs

- No renaming: a misspelt shelf is fixed by putting its books on a new one. Revisit if asked.
