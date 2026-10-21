## Context

The interface language is chosen once at start from the system (`i18n::set_language`, called by the GTK shell from
`glib::language_names()` and by the Mac shell from `Locale.preferredLanguages` through `forzac_set_language`), before
any string is asked for; menus, panels and the library take their strings when they are built. Sync (`sync::sync`)
runs off the UI thread when the library shows, on focus changes and on quit; nothing remembers how it ended, and the
`device` line's name is `$HOSTNAME` or `$USER` (on a Mac the user's short name: two Macs look the same). The Android
app's settings screen (its `app-settings`) shows the same two sections.

## Goals / Non-Goals

**Goals:** a reader can have Forzac in Russian on an English system and back; a reader can see how sync works, whether
it works, and which devices it reaches, and sync on demand.

**Non-Goals:** switching the language without a restart; sync options; a settings window reachable from a book on
Linux.

## Decisions

**1. The language choice is a state setting, applied at start (core).** `i18n::SETTING` (`language`) holds `en`, `ru`
or nothing ("As the system"). `Language::choose(setting, locales)` is `from_locales` over the setting followed by the
system's locales, so the choice simply comes first. The GTK shell opens the state before choosing (it already opens it
in `main`), the Mac shell calls `forzac_choose_language(state, locales)` right after opening the state in
`applicationWillFinishLaunching`, before the menus are built; runs without a state keep `forzac_set_language`. Live
switching was rejected: every shell string is taken when its view is built (menus, the library, open readers, panels),
so it would mean rebuilding every window; the window says a restart applies it.

**2. The last sync is remembered in the process (core).** `sync::sync` records the time it ended and the error, if
any, in a process-wide slot (`sync::last()`); every path (library, focus, quit, "Sync Now", either shell) goes through
it. Not kept in the state: writing the state on every focus change for a line in a window is not worth it, and after a
start the library's sync fills it at once.

**3. The other devices are read from the files' first lines (core).** `sync::devices(folder, own)` reads, for each
`*.sync` file in `.forzac/` but the own, only up to its `device` line, and the file's modification time — the last time
that device wrote something new. The latest first. Reading is done off the UI thread (an iCloud placeholder can block
on download), on a state handle of its own, like the other folder reads.

**4. The window's sync text comes from the core (core → C ABI).** `sync::status(store, now, offset)` gives the Books
folder, the state line ("Synced {when}", "Couldn't sync: {reason}", "Not synced yet") and the devices with "{when}",
already in the interface language. `when(time, now, offset)` turns milliseconds into "today at 10:14", "yesterday at
…", "21 October at …", "21 October 2025 at …" in the local time zone, the shell passing its current UTC offset (a date
on the other side of a daylight-saving change is an hour off; acceptable for "when did it last sync"). Months are in the
catalogue (Russian genitive). The C ABI: `forzac_sync_status(state, offset)` returns lines — the folder, the state
line, then `name\twhen` per device — valid until the next call on that handle; "Syncing…" is the shell's own, while its
"Sync Now" runs.

**5. The device name comes from the shell.** `sync::set_device_name` (C ABI `forzac_set_device_name`) at start: macOS
`SCDynamicStoreCopyComputerName` (the name in Sharing settings, as the Finder shows it), GTK `glib::host_name()`. The
environment stays the fallback. The name is written the next time the own file changes; the file's text otherwise
stays the same, so no new conflicts.

**6. The windows (shells).** macOS: a window of its own (`macos/Settings.swift`), standard AppKit controls in the
system's appearance — a pop-up button for the language, wrapping labels, push buttons — not a sheet over the library:
`⌘,` opens it without the library in front, and a Mac's settings window is a window. GTK: a sheet over the library in
its theme, built as "About the book" is (a backdrop in the overlay, CSS classes of the library's sheets), a
`gtk::DropDown` for the language. "Change…" reuses each shell's "Change the Books folder…" (Mac:
`AppDelegate.changeBooksFolder`, which shows the library when it is closed; GTK: `Library::choose_books_folder`), and
the window reads its sync text again when the change ends. "Sync Now" reuses the shells' sync: Mac `AppDelegate.sync`
(saves the open books' pending moves, moves open books to newer places) with a completion; GTK `sync_state` off the
main thread (in GTK the library is shown when no book window is open).

**7. `⌘,` by the key window (Mac).** Agreed with the maintainer: Forzac › Settings… (`⌘,`) opens the "Aa" panel when a
book window is key, else the settings window. The library's "⋯" menu always opens the settings window.

## Risks / Trade-offs

- [A restart for the language] → said in the window; the choice is saved at once.
- [The device list reads every other device's file head] → a handful of files, first lines only, off the UI thread.
- [The current UTC offset for older dates] → at most an hour off across a daylight-saving change.
- [GTK cannot be built on macOS] → built in `ci/fedora.Dockerfile`, with clippy installed in the container.
