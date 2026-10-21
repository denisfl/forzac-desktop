# The sync format

How Forzac apps share reading data through a folder synced by Dropbox, iCloud Drive, Google Drive or Syncthing, with
no server of their own (`openspec/specs/sync`). The desktop (Rust) implements it in `core/src/sync.rs`; the Android app
(Kotlin) is to implement the same.

## Where

`<Books folder>/.forzac/<device>.sync`, one file per device. `<device>` is 16 lowercase hex digits made once per
installation. A device writes only its own file — so a sync service never sees two writers of one file and makes no
"conflicted copy" — and reads every `*.sync` file in the folder.

## The file

UTF-8 text, `\n` line ends. The first line is `forzac-sync 1`. Every other line is fields separated by tabs; in a field
a tab is written `\t`, a line break `\n`, a backslash `\\`. Unknown or damaged lines are skipped (and a newer version may
add fields at the end of a line: readers ignore extra fields).

```text
forzac-sync 1
device	<id>	<name>
position	<book>	<time>	<place>	<progress>
bookmark	<book>	<time>	<deleted>	<place>
highlight	<book>	<time>	<deleted>	<place>	<place>
shelf	<book>	<time>	<deleted>	<name>
edit	<book>	<time>	<title>	<authors>	<series>	<number>	<genres>
```

- `<book>`: the book's id — the first 16 bytes of the SHA-256 of its file, 32 lowercase hex digits (both apps already
  name books so).
- `<time>`: when the item last changed, milliseconds since the Unix epoch.
- `<deleted>`: `1` for a removed item (a tombstone, kept so the removal reaches every device), else `0`.
- `<place>`: four fields — `chapter`, `block`, `offset`, `fragment`. Chapter and block are 0-based indices in the book
  as both parsers build it; `offset` counts Unicode code points (not UTF-8 bytes, not UTF-16 units) into the block's
  text; `fragment` is up to 24 code points of the text from that place, to find it again when the two parsers number
  blocks differently.
- `<progress>`: the share read, 0 to 1, decimal with a dot.
- `<name>`: a shelf's name as the reader typed it.
- `edit`: the reader's edits of a book's details; each field is `-` for "the book's own" or `=` followed by the value
  (`=` alone: empty — for `series` that means "no series"); `authors` and `genres` are separated by `, `.

A device's own file may hold device-local fields after the documented ones (the desktop keeps the place in its own
coordinates there); other devices ignore them.

## Keys and merging

An item's key: `position` — the book; `bookmark` — the book and its place's `chapter.block.offset`; `highlight` — the
book and both places' `chapter.block.offset`; `shelf` — the book and the name in lower case; `edit` — the book. The key
of a bookmark or highlight is fixed by the device that made it and is kept by every device that copies the item.

For every key, the item with the latest `<time>` across all files wins (a tie: either). A device writes into its own
file every item it knows — its own changes with the time of the change, the others' with their times — so the files
converge. Tombstones are kept.

Clocks differ between devices, so the time of a local change is the time of the change but at least one millisecond
later than the item it replaces in the own file (`max(now, seen + 1)`): a change made after taking an item from another
device is newer than that item whatever the clocks say. A device finds its local changes by comparing its state with
its own file: a place it applied that is no longer the book's place is a local change; a place it never applied (the
book was missing) is replaced only by a later reading.

A device remembers a digest of what it last wrote to its own file. When the file is there and holds something else,
another installation with the same id (a copied state) wrote it: the device takes a new id and starts a new own file;
the old one counts as another device's.

A device syncs when it is left (it gives its changes away) and when it is come back to (it takes the others'): on a
phone when the app leaves and enters the screen, on a computer when the app loses and gains focus, and on quitting.

## Finding a place

To use a remote place in a book: if block `block` of chapter `chapter` exists and its text at `offset` starts with
`fragment`, that is the place; else the first occurrence of `fragment` in the book, searched from that block outwards,
is the place; else the place is the start of that block (clamped to the book).
