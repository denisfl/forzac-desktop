# Design

## Context

The state file keeps positions (`book` lines, `opened` in seconds), bookmarks, highlights, shelves and edits by book id;
`Locator` offsets are UTF-8 bytes. Android's are UTF-16 units, and its parser is a separate implementation. Both shells
read the Books folder on a background thread before showing the library (`books_folder`, `scan`).

## Decisions

**1. In the Books folder, one file per device.** `.forzac/<device>.sync`; the device id is the setting `sync.device`
(16 hex digits from the SHA-256 of the time, the process id and the state's path). One writer per file removes sync
conflicts; reading all files gives each device the whole picture.

**2. Snapshot files with times, last change wins.** Every file holds every item its device knows, with the time of the
item's last change. Merge: per key, the latest time. The own file also keeps, after the documented fields, the place in
this device's coordinates (`chapter:block:byte offset`), which is how local changes are found without a change log in
the state: a bookmark in the state whose local place is not in the own file is new (time now); an item in the own file
whose local place is no longer in the state was removed (a tombstone, time now); a position is newer when the state's
`opened` (seconds) is later than its item's time.

**3. Parsing only what must be.** Converting between local `Locator`s and portable places needs the book's text, so a
book is parsed only when one of its local marks or position changed, or a remote item for it won and must be applied.
A sync with nothing new reads the small files and writes nothing.

**4. Places with fragments.** Code-point offsets work for both UTF-8 (Rust) and UTF-16 (Kotlin). The fragment (24 code
points) recovers from differing block numbering: exact match at the indices first, then a search outwards from the
block, then the block's start.

**5. When.** `sync::sync(folder, store)` is called in the shells' library loading, before `scan`, on the background
thread. Positions written while reading reach the own file the next time the library shows (non-goal: live sync).

## Risks / Trade-offs

- Clock skew between devices decides ties of "latest"; acceptable for one reader's devices.
- Tombstones grow without bound; a few bytes per removal.
