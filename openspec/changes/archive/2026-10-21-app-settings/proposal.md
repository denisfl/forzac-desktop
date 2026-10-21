## Why

Two questions have no answer in the app today. The maintainer's Mac is in English and they want Forzac in Russian:
the interface follows the system's preferred languages and nothing else. And syncing (`sync`) works without a word:
nothing says it needs the Books folder kept by Dropbox, iCloud Drive or Syncthing (on Android only Syncthing keeps a
folder), when the last sync was, whether it failed, or which other devices this one sees. The Android app is getting a
"Settings" screen with the same two things (its `app-settings`); the desktop gets a small window to match.

A good default is not enough for the language: the default stays "As the system", but a reader whose system language
is not the one they want to read menus in has no other way. Sync has no setting of its own here — the window explains
it and shows its state; the Books folder was already a setting (`books-folder-setting`).

## What Changes

- "Settings…" ("Настройки…") in the library head's "⋯" menu opens one small settings window: on macOS a window of its
  own, on Linux a sheet over the library.
- Language: "As the system" (default), "English", "Русский". Kept in the state; at start the choice wins over the
  system's languages. It takes effect at the next start, and the window says so.
- Sync: a short explanation of how it works and what syncs; the Books folder with "Change…" (the existing flow); the
  last sync's time or its error; the other devices seen in `.forzac/` with when each last wrote; "Sync Now", the same
  sync that runs when the app is left and come back to.
- The device name written into the sync file is the computer's name (macOS: the name in Sharing settings; Linux: the
  host name) instead of `$HOSTNAME` or `$USER`, so two Macs of one user are told apart in the list.
- macOS: `⌘,` and Forzac › Settings… open the settings window when no book window is in front; in a book window they
  still open the reader's "Aa" panel.

## Non-goals

- Switching the language live (every window, menu and panel would have to be rebuilt).
- A sync on/off switch, a choice of service, a sync interval, or anything about the sync folder besides the Books
  folder itself.
- Moving the reader's text settings ("Aa") into this window.
- A keyboard shortcut for the window on Linux.

## Capabilities

### New Capabilities
- `settings`: the settings window — language and sync.

### Modified Capabilities
- `localization`: the reader's choice of language wins over the system's.
- `library`: "Settings…" in the library's menu.
- `sync`: the device's name in its file.
- `macos-app`: what `⌘,` opens.

## Impact

Core: `core/src/i18n.rs` (the choice and its setting), `core/src/sync.rs` (the last sync's outcome, the other devices,
the device name, the time as words), `core/src/ffi.rs` and `macos/forzac.h`. macOS: a new `macos/Settings.swift`,
`macos/main.swift` (language at start, `⌘,`, "Sync Now"), `macos/Library.swift` (the menu item). GTK: `gtk/src/main.rs`
(language at start, device name), `gtk/src/library.rs` (the menu item and the sheet). The catalogue gets the window's
strings. `docs/sync-format.md`: the name.
