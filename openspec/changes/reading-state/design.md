## Context

The shells open a book at chapter 0 (or `--chapter N`) and keep the reading position as a `Locator` in memory; the
theme and text size reset at every launch. GTK runs one process per user (GApplication); the macOS shell is one
process. The core has no platform code and is tested on its own.

## Goals / Non-Goals

**Goals:** remember positions, recent books and settings; restore them on open; a recent-books list in the GTK shell.

**Non-Goals:** a library screen with covers and import; syncing; the recent-books list in the macOS shell (it comes with
`macos-app`, which brings the Mac shell up to the Linux one); window size and position.

## Decisions

### A text file, not SQLite
The state is small (an entry per book ever opened, a few settings) and written rarely. A text file rewritten atomically
is enough and needs no dependency; SQLite would add a C library to build on every platform for no gain. The file is
readable and can be fixed by hand:

```
forzac-state 1
setting<TAB>theme<TAB>Dark
book<TAB>id<TAB>chapter<TAB>block<TAB>offset<TAB>progress<TAB>opened<TAB>path<TAB>title<TAB>authors
```

Fields are tab-separated; tabs, line breaks and backslashes inside them are escaped (`\t`, `\n`, `\\`). Unknown or
malformed lines are skipped. A write goes to `state.tmp` and is renamed over `state`.

### The core owns the store; shells give it the path
`forzac_core::state::Store` reads, queries and writes the file at a path the shell passes: GTK uses
`glib::user_data_dir()/forzac/state`, macOS the Application Support directory. Every write re-reads the file and
changes only the entries it is writing, so two processes (or two windows) do not undo each other.

### Book identity
`Book::id` is computed in `parse` from the bytes: FNV-1a 64 over the length, the first 64 KiB and the last 64 KiB.
Cheap for any book size (hashing a whole 10 MB EPUB would cost tens of milliseconds on the Pi), stable across Rust
versions (unlike `DefaultHasher`), and a collision needs two books of the same size with the same start and end. The
path is stored too, as a fallback when the content changed and for the recent list.

### Progress
The share of the book's text before the position: the characters of all blocks before it plus the offset in its
block, over all characters. Computed in the core when the position is saved.

### Writing
The GTK view reports position changes; the window restarts a 1 s timer on each and saves when it fires and on
`close-request`. The page-turn animation is 200 ms, so a save never lands inside one. Settings are saved at once when
changed (they change rarely). The macOS shell does the same with a `Timer` and `windowWillClose`, through new C ABI
calls (`forzac_state_*`).

### Recent books (GTK)
`activate` without files opens a small window with a `GtkListBox`: rows of title, authors and progress; arrows move
natively, `j`/`k` are added, `Enter` (`row-activated`) opens the book in a reader window and closes the list.

## Risks / Trade-offs

- A changed file with the same size, start and end keeps its old position → harmless: the Locator is clamped to the
  book.
- Positions of books never reopened accumulate → a few hundred bytes per book; no pruning before it matters.
