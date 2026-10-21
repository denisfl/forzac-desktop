## Context

`sync::sync` (sync-format) finds local changes by comparing the state with the own file and is called by the shells
only when the library shows. Positions are saved a second after a move (Mac) or `SAVE_DELAY` after it (GTK) and when
a book window closes; `save_position` stamps `opened` with the time of the save, so a position's time is its last move.

## Goals / Non-Goals

**Goals:** a place reaches the folder when the reader leaves the device and reaches an open book when they come back;
"latest" survives a wrong clock; one file per device survives a copied state.

**Non-Goals:** syncing while reading on one device (nobody reads on the other one meanwhile); watching the folder for
changes; iCloud/Dropbox placeholders (next change); matching one book in two different files.

## Decisions

**1. Leave and come back, not a timer.** Reading moves between devices at the moment one is left; the app's focus is
that moment on a computer, as the screen is on a phone. Mac: `didResignActive` → save the open books' places, sync;
`didBecomeActive` → sync, then each book window whose stored place changed goes there; `applicationWillTerminate` →
save, sync on the calling thread. GTK has no app focus: a book window's focus change does the same (switching between
Forzac's own windows syncs too — cheap: files are read, a book is parsed only for a new place), and the application's
`shutdown` syncs. The library still syncs when it shows. Syncs run on the existing background queue/thread with a state
handle of their own, one after another. Only a move still waiting to be saved is saved before a sync: saving an unmoved book would
stamp its place with the time of the click that brought the app back and make it win over the place read since on the
phone (found on the device).

**2. Which books moved.** `sync` returns the ids whose position it set; the shells compare the stored place of an open
book with the place they last saved and go there when it differs (`go(to:)`/`go_to` keep the place left for "back").
The C ABI gets `forzac_synced(state)`: the ids the last sync on that handle moved, one a line.

**3. Times: `max(now, seen + 1)`.** A Lamport-style bump on the wall clock: a local change replacing an item in the own
file gets a time later than that item, so taking a place from a device whose clock is ahead and then reading on is
still "later". A position's local change is now "the own file's applied place differs from the book's", not "and its
time is later": the time condition was there for places that were never applied here (the book was missing), which keep
it.

**4. A copied device.** The state remembers a digest of the text it last wrote to its own file (`sync.written`). If the
file exists, is readable and has another digest, another installation with the same id wrote it: take a new id, start
the own file empty (the state's items become new local changes; the other file is merged as a device's). A missing or
unreadable file (an iCloud placeholder) is not a copy. A crash between writing the file and the setting at worst
renames this device once.

## Risks / Trade-offs

- [Syncs on every focus change] → reading a few small files; the own file is written only when its text changes.
- [A jump the reader did not ask for] → only when another device has a later place; "back" returns.
- [GTK cannot be built on macOS] → the GTK shell is built and run in `ci/fedora.Dockerfile`.
