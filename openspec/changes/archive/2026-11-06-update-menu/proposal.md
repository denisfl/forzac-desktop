## Why

Forzac asks GitHub for a new release at most once a day, when the library opens. A release published just after a check stays unseen for up to a day: 0.20.0 came out eight minutes after the maintainer's Forzac had checked, and the maintainer saw no update the next morning. There is no way to ask now. The line also reads "Version 0.20.0 is out — install", where a sentence and a button read better: "Version 0.20.0 is out. Install".

## What Changes

- "Check for Updates…" in the application menu (macOS: Forzac menu; Linux: the library's main menu, not in builds from a package manager): asks at once and answers in a dialog — a newer version with "Install"/"Download" and "Later", "the latest version", or "couldn't check".
- The line in the library: "Version 0.20.0 is out. Install" (Linux "Download"), Russian "Вышла версия 0.20.0. Установить" ("Скачать").

No setting is added: the daily check stays as it is.

## Capabilities

### New Capabilities
<!-- none -->

### Modified Capabilities
- `update-check`: the line's wording; checking by hand.

## Impact

`core/src/update.rs` (`record_asked`), `core/src/ffi.rs` and `macos/forzac.h` (`forzac_update_asked`, `forzac_update_version`), `core/src/i18n.rs`; `macos/main.swift` (the menu item, the dialog), `macos/Library.swift` (the line); `gtk/src/library.rs` (the menu item, the dialog, the line).
