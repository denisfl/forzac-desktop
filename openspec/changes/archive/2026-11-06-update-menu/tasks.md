## 1. Core

- [x] 1.1 `update::record_asked`: remembers the answer, offers a newer version again even if dismissed; the strings in `i18n` — check: core tests
- [x] 1.2 `forzac_update_asked`, `forzac_update_version` in the C ABI and `forzac.h` — check: the core test of `record_asked`; the shells build against it

## 2. Shells

- [x] 2.1 (2026-11-06: the line on screenshots in English and Russian; the menu and its dialog checked by the maintainer on the Mac) macOS: Forzac › Check for Updates…, the dialog, "Install" from it; the line's new wording — check: a screenshot of the line and of each dialog
- [x] 2.2 (2026-11-06: built and linted; not seen on screen) Linux: the main menu's item (not in a packaged build), the dialog, "Download"; the line's wording — check: `clippy`, a screenshot under headless Weston

## 3. Check

- [x] 3.1 (2026-11-06: 373 tests; by hand on the Mac by the maintainer) `cargo test -p forzac-core`, `clippy`, `fmt`, `openspec validate update-menu --strict`; by hand on the Mac against the real release
