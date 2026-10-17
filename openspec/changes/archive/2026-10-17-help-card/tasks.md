## 1. Core

- [x] 1.1 `i18n`: the help as one table (Linux keys, macOS keys, meaning; sections; title; support line) and
      `help(platform) -> Help`; the Russian rows in the catalogue; the old text blocks removed; tests (rows per platform,
      Share only on the Mac, the Russian help has every row, the completeness test covers the table)
- [x] 1.2 C ABI: `forzac_help` returns the flattened help; `macos/forzac.h` comment; test

## 2. GTK shell

- [x] 2.1 `gtk/src/help.rs`: dimming, card, head with the close button, sections in columns, key caps, foot with the
      link; theme CSS
- [x] 2.2 `main.rs`: the help instead of the label; keys while open (hide, scroll, swallow); clicks; fade

## 3. macOS shell

- [x] 3.1 `macos/Help.swift`: the card drawn and laid out as in GTK, theme colours, scrolling, the link
- [x] 3.2 `main.swift`: the help instead of the text field; keys while open; clicks; fade; `--panel help`

## 4. Docs

- [x] 4.1 `docs/roadmap.md` (the change in the plan), README if it shows the help

## 5. Verification

- [x] 5.1 `cargo fmt`, `clippy -D warnings`, `cargo test -p forzac-core`; GTK builds in the `ci/debian` container;
      `macos/build.sh`
- [x] 5.2 Screenshots in English and Russian on the Mac and GTK (container): a 760 × 980 window (one column, scrolled
      with `j`) and a wide one (two columns), a light and a dark theme
- [ ] 5.3 By hand: `?`/`Esc`/the close button/a click beside the card hide it; other keys do nothing while it is open;
      the link opens the site; idle 0 CPU with the help open
