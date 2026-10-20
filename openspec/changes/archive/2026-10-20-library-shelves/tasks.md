# Tasks

## 1. Core

- [x] 1.1 State: `shelf` lines, `shelves_of`, `shelf_names`, `set_shelf`, kept by other writes and by `forget`; tests
- [x] 1.2 `Entry::shelves` from `scan`; `View::Shelves` in `arrange` (automatic, then the reader's); tests
- [x] 1.3 Strings; C ABI (`ForzacRecent` and `ForzacBookKey` with shelves, `forzac_shelf_names`, `forzac_set_shelf`),
      `macos/forzac.h`; tests

## 2. Shells

- [x] 2.1 GTK: the tab, "Add to shelf" with checks and "New shelf…", the name's sheet
- [x] 2.2 macOS: the same
- [x] 2.3 Both builds and clippy green; by hand on the Mac in Russian over ~/Books (a state of its own): "New shelf…"
      in an alert, a second book put on it from the submenu, the check, taking it off, the Shelves view after a restart.
      GTK: the Shelves view as a PNG in the Debian container in Russian; its submenu and sheet only compiled
- [x] 2.4 `docs/roadmap.md` status
