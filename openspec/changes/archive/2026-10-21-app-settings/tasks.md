## 1. Core

- [x] 1.1 `i18n::SETTING`, `Language::code`, `Language::choose`; the C ABI `forzac_choose_language`, `forzac_language_choice`, `forzac_set_language_choice`
- [x] 1.2 `sync::last()` (the last sync's end and error), `sync::set_device_name`, `sync::devices`, `sync::when`, `sync::status`; the C ABI `forzac_set_device_name`, `forzac_sync_status`; `forzac.h`
- [x] 1.3 The window's strings and the months in the catalogue
- [x] 1.4 Tests: the choice over the system's languages, `when` (today, yesterday, this year, before, a time zone; English — the Russian words are checked by the catalogue test), devices (own skipped, empty name, not a sync file, latest first), the state line
- [x] 1.5 `docs/sync-format.md`: the device name

## 2. macOS

- [x] 2.1 The language from the state at start; the computer's name for sync
- [x] 2.2 `macos/Settings.swift`: the window (language, sync explanation, folder with "Change…", state, devices, "Sync Now"); `Esc` and `⌘W` close it
- [x] 2.3 "Settings…" in the library's "⋯" menu; `⌘,` by the key window; the window read again after a sync or a folder change

## 3. GTK

- [x] 3.1 The language from the state at start; the host name for sync
- [x] 3.2 The settings sheet over the library and "Settings…" in the "⋯" menu; `Esc`, "Close" and a click beside close it

## 4. Check

- [x] 4.1 `cargo test -p forzac-core`, `cargo clippy -p forzac-core --all-targets -- -D warnings`, `cargo fmt --all --check`, `macos/build.sh`
- [x] 4.2 The GTK shell builds and passes clippy in `ci/fedora.Dockerfile` (clippy installed in the container; the sheet was not run)
- [x] 4.3 On the Mac with the maintainer (2026-10-21, dev build on a test state): "Settings…" from "⋯" and `⌘,` (library vs book), Russian on an English system after a restart and back, "Sync Now", the phone in "Other devices", "Change…"
