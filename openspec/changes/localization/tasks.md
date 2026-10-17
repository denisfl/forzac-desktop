## 1. Core

- [ ] 1.1 `i18n`: language from locales, `tr`, `tr_count` (Russian plurals), `fill`, the help texts; "Chapter N" and
      "Finished" through it; tests
- [ ] 1.2 C ABI and `macos/forzac.h`
- [ ] 1.3 The completeness test over both shells' sources

## 2. Shells

- [ ] 2.1 GTK: every visible string through `tr`; the language from GLib at start
- [ ] 2.2 macOS: every visible string through `tr`, the menus; the language from `Locale`; `CFBundleLocalizations`

## 3. Docs

- [ ] 3.1 README (the language), `docs/android-parity.md` X5, roadmap row 12

## 4. Verification

- [ ] 4.1 fmt, clippy, tests, the corpus test; both shells build
- [ ] 4.2 Pi 5: `scripts/pi-headless.sh` in English and with `LANG=ru_RU.UTF-8` (first page, turns, idle); every
      screen in Russian and English: page, end page, contents and its three tabs, search, settings, note, bars, pill,
      recent books, the message for a non-book, help; screenshots
- [ ] 4.3 Mac: the same in both languages (`-AppleLanguages`), and the menus; screenshots
