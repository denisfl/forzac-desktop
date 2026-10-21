## Why

Reading moves between devices: a chapter on the Mac, then the phone on the sofa. Today the desktop syncs only when the
library shows, so a place read on the Mac reaches the folder only after the reader goes back to the library — usually
never before picking up the phone; and a place read on the phone reaches an open book on the Mac only after the
library is shown again. Two quieter cases also break "the latest change wins": a device whose clock is behind loses
its own later changes to an item it has just taken from another device, and two computers with one device id (a
Migration Assistant copy, a restored backup) write one file, which the sync service turns into conflicted copies.

## What Changes

- Hand-over: the desktop gives its changes away when it is left (the app or a book window loses focus, a book closes,
  the app quits) and takes the others' when it comes back (the app becomes active); an open book whose place came from
  another device goes there, keeping the place left for "back".
- Times that respect what a device has seen: a change made after taking an item from another device is always newer
  than that item, whatever the clocks say (`time = max(now, seen + 1)`); a place held in the own file that is no longer
  the book's place is a local change, whatever its time.
- A copied device notices it: when the own file holds something this device did not write, it takes a new id.
- `docs/sync-format.md` states the three rules, so the Android app follows them.

## Capabilities

### New Capabilities

### Modified Capabilities
- `sync`: when it syncs; times of local changes; copied devices.

## Impact

`core/src/sync.rs` (times, the copied-device check, the books whose place moved), `core/src/ffi.rs` (that list for
Swift), `macos/main.swift` (app activation, resignation, quit), `gtk/src/main.rs` (book window focus, shutdown),
`docs/sync-format.md`. No new settings.
