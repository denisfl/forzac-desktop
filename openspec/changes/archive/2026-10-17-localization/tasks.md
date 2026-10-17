## 1. Core

- [x] 1.1 `i18n`: language from locales, `tr`, `tr_count` (Russian plurals), `fill`, the help texts; "Chapter N" and
      "Finished" through it; tests
- [x] 1.2 C ABI and `macos/forzac.h`
- [x] 1.3 The completeness test over both shells' sources

## 2. Shells

- [x] 2.1 GTK: every visible string through `tr`; the language from GLib at start
- [x] 2.2 macOS: every visible string through `tr`, the menus; the language from `Locale`; `CFBundleLocalizations`

## 3. Docs

- [x] 3.1 README (the language), `docs/android-parity.md` X5 (the roadmap lists changes without a status)

## 4. Verification

- [x] 4.1 fmt, clippy, tests, the corpus test; both shells build
- [x] 4.2 Pi 5: `scripts/pi-headless.sh` in English and with `LANG=ru_RU.UTF-8` (first page, turns, idle); every
      screen in Russian and English: page, end page, contents and its three tabs, search, settings, note, bars, pill,
      recent books, the message for a non-book, help; screenshots
- [x] 4.3 Mac: the same in both languages (`-AppleLanguages`), and the menus; screenshots

Recorded in `docs/android-parity.md` ("The interface in Russian"). Not checked by hand: the Mac's alert for a file that
cannot be opened.
